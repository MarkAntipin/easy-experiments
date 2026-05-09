use std::path::Path;

use duckdb::Connection;

const SCHEMA_SQL: &str = include_str!("../../migrations/duckdb/0001_init.sql");

/// Open the shared DuckDB `Database` and ensure the analytics tables exist.
///
/// The whole process funnels through a single `Database` instance — the
/// returned `Connection` owns it, and writers/readers receive their own
/// sessions via `Connection::try_clone`. Sharing one engine gets us in-process
/// MVCC, a single buffer pool, and a single file lock instead of three
/// independent `Database`s racing on the same file.
pub fn open_and_bootstrap(path: &Path) -> duckdb::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(conn)
}
