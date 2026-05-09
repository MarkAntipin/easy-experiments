use std::sync::{Arc, Mutex};

use duckdb::Connection;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::errors::CustomError;

/// Per-connection memory cap. Aggregation queries can scan millions of rows;
/// we cap each so a runaway can't OOM the whole process.
const PER_CONN_MEMORY_LIMIT: &str = "256MB";

/// Bounded pool of DuckDB connections backed by a single shared `Database`.
///
/// All pooled connections are produced via `Connection::try_clone` from a
/// template held inside the pool, so they share the same engine, buffer pool,
/// catalog, and WAL as the writer connections — that's how we get correct
/// in-process MVCC and read-after-write across handoffs.
///
/// Each handout is gated by a semaphore so we never exceed `max_size`
/// concurrent queries; idle connections live on a LIFO stack so a hot
/// connection's caches stay warm. Connections are created lazily on first
/// acquire that finds the idle stack empty.
pub struct DuckDBReadPool {
    permits: Arc<Semaphore>,
    idle: Arc<Mutex<Vec<Connection>>>,
    template: Arc<Mutex<Connection>>,
    max_size: usize,
}

impl DuckDBReadPool {
    /// Build a pool that clones from `template`. The template itself stays in
    /// the pool for future lazy expansion.
    pub fn new(template: Connection, max_size: usize) -> Self {
        let max_size = max_size.max(1);
        Self {
            permits: Arc::new(Semaphore::new(max_size)),
            idle: Arc::new(Mutex::new(Vec::with_capacity(max_size))),
            template: Arc::new(Mutex::new(template)),
            max_size,
        }
    }

    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Acquire a connection. Awaits a permit; clones a new connection off the
    /// shared `Database` only if no idle one is available. The returned guard
    /// returns the connection to the pool on drop.
    pub async fn acquire(&self) -> Result<PooledConnection, CustomError> {
        let permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| CustomError::InternalError("analytics pool closed".to_string()))?;

        let maybe_conn = {
            let mut idle = self.idle.lock().expect("analytics pool mutex poisoned");
            idle.pop()
        };
        let conn = match maybe_conn {
            Some(c) => c,
            None => clone_from_template(&self.template)?,
        };

        Ok(PooledConnection {
            conn: Some(conn),
            idle: Arc::clone(&self.idle),
            _permit: permit,
        })
    }
}

fn clone_from_template(template: &Mutex<Connection>) -> Result<Connection, CustomError> {
    let template = template.lock().expect("analytics pool template poisoned");
    let conn = template.try_clone().map_err(|e| {
        CustomError::InternalError(format!("failed to clone DuckDB read connection: {}", e))
    })?;
    let pragma = format!("SET memory_limit='{}'", PER_CONN_MEMORY_LIMIT);
    if let Err(e) = conn.execute_batch(&pragma) {
        tracing::warn!(error = %e, "analytics: failed to set DuckDB memory_limit");
    }
    Ok(conn)
}

pub struct PooledConnection {
    /// `Option` so `Drop` can take the connection back into the pool while we
    /// still own the value (`drop` only gets `&mut self`).
    conn: Option<Connection>,
    idle: Arc<Mutex<Vec<Connection>>>,
    _permit: OwnedSemaphorePermit,
}

impl PooledConnection {
    pub fn conn(&self) -> &Connection {
        self.conn
            .as_ref()
            .expect("PooledConnection used after drop")
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            if let Ok(mut idle) = self.idle.lock() {
                idle.push(conn);
            }
            // If the mutex is poisoned, drop the conn — pool stays usable.
        }
    }
}
