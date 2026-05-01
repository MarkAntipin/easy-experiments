use std::path::PathBuf;
use std::time::{Duration, Instant};

use duckdb::{params, Connection};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::event::ExposureEvent;

#[derive(Debug, Clone, Copy)]
pub struct WriterConfig {
    pub batch_capacity: usize,
    pub flush_interval: Duration,
}

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS exposures (
    event_id        VARCHAR NOT NULL,
    schema_version  INTEGER NOT NULL,
    ts_ms           BIGINT  NOT NULL,
    company_id      VARCHAR NOT NULL,
    experiment_id   VARCHAR NOT NULL,
    variant_key     VARCHAR,
    entity_id       VARCHAR NOT NULL
);
";

pub fn spawn_writer(
    rx: mpsc::Receiver<ExposureEvent>,
    duckdb_path: PathBuf,
    config: WriterConfig,
) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || run_writer(rx, duckdb_path, config))
}

fn run_writer(
    mut rx: mpsc::Receiver<ExposureEvent>,
    duckdb_path: PathBuf,
    config: WriterConfig,
) {
    let conn = match Connection::open(&duckdb_path) {
        Ok(c) => c,
        Err(e) => {
            log::error!(
                "analytics: failed to open DuckDB at {:?}: {}",
                duckdb_path, e
            );
            drain_until_closed(&mut rx);
            return;
        }
    };

    if let Err(e) = conn.execute_batch(SCHEMA_SQL) {
        log::error!("analytics: failed to bootstrap DuckDB schema: {}", e);
        drain_until_closed(&mut rx);
        return;
    }

    if let Err(e) = conn.execute_batch("SET memory_limit='256MB'") {
        log::warn!("analytics: failed to set DuckDB memory_limit: {}", e);
    }

    let runtime = tokio::runtime::Handle::current();
    let mut buf: Vec<ExposureEvent> = Vec::with_capacity(config.batch_capacity);
    let mut last_flush = Instant::now();

    loop {
        let elapsed = last_flush.elapsed();
        let should_flush = buf.len() >= config.batch_capacity
            || (!buf.is_empty() && elapsed >= config.flush_interval);

        if should_flush {
            if let Err(e) = flush_batch(&conn, &mut buf) {
                log::error!("analytics: DuckDB flush failed, dropping {} events: {}", buf.len(), e);
                buf.clear();
            }
            last_flush = Instant::now();
            continue;
        }

        let timeout = config.flush_interval.saturating_sub(elapsed);
        let want = config.batch_capacity.saturating_sub(buf.len()).max(1);

        let n = runtime.block_on(async {
            tokio::time::timeout(timeout, rx.recv_many(&mut buf, want)).await
        });

        match n {
            Ok(0) => {
                // All senders dropped. Flush whatever's left and exit.
                if !buf.is_empty() {
                    if let Err(e) = flush_batch(&conn, &mut buf) {
                        log::error!(
                            "analytics: final DuckDB flush failed, dropping {} events: {}",
                            buf.len(), e
                        );
                    }
                }
                return;
            }
            Ok(_) => { /* events appended to buf */ }
            Err(_) => { /* timeout — top of loop will flush */ }
        }
    }
}

fn flush_batch(conn: &Connection, buf: &mut Vec<ExposureEvent>) -> duckdb::Result<()> {
    let mut app = conn.appender("exposures")?;
    for ev in buf.iter() {
        app.append_row(params![
            ev.event_id.to_string(),
            ev.schema_version as i32,
            ev.ts_ms,
            ev.company_id,
            ev.experiment_id,
            ev.variant_key,
            ev.entity_id,
        ])?;
    }
    app.flush()?;
    buf.clear();
    Ok(())
}

/// If we can't write to DuckDB at all, keep draining the channel so producers
/// don't fill it and start dropping. Events are silently discarded.
fn drain_until_closed(rx: &mut mpsc::Receiver<ExposureEvent>) {
    while rx.blocking_recv().is_some() {}
}
