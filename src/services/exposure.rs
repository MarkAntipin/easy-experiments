use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use duckdb::{params, Connection};
use moka::sync::Cache as SyncCache;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::models::ExposureEvent;

/// Abstraction for "where do exposure events go".
///
/// Today's only real impl is `MpscEventSink`, which hands events to a local
/// DuckDB writer task. Tomorrow's impl could be a Kafka producer, an S3
/// Parquet batcher, etc. Handlers depend only on this trait.
pub trait EventSink: Send + Sync + 'static {
    fn record_exposure(&self, event: ExposureEvent);
    fn dropped(&self) -> u64;
    fn deduped(&self) -> u64 {
        0
    }
}

/// Controls server-side exposure dedup.
///
/// We dedup at the sink so that callers hammering `/evaluate` for the same
/// (company, experiment, entity) don't write a fresh exposure row per call —
/// only the first call within `ttl` is recorded. This is the core mechanism
/// that makes a "dumb client" architecture viable: clients can call as often
/// as they like, the server collapses repeats into one event per session.
#[derive(Debug, Clone, Copy)]
pub struct DedupConfig {
    pub max_capacity: u64,
    pub ttl: Duration,
}

impl Default for DedupConfig {
    fn default() -> Self {
        Self {
            // ~1M unique (company, experiment, entity) tuples in memory at any
            // time. At ~150 bytes per entry that's ~150MB worst case.
            max_capacity: 1_000_000,
            // 1h matches a typical "session-ish" granularity. Same user
            // returning tomorrow gets a fresh exposure row.
            ttl: Duration::from_secs(60 * 60),
        }
    }
}

type DedupKey = (String, String, String);

/// In-process bounded mpsc sink with server-side dedup. Non-blocking on
/// `record_exposure`: dedup hits, send failures, and successful sends all
/// return immediately. The dedup cache is consulted before the channel send,
/// so a saturated channel never causes a thundering herd of retries — each
/// (company, experiment, entity) tuple has at most one chance per TTL window
/// to land in the writer.
pub struct MpscEventSink {
    tx: mpsc::Sender<ExposureEvent>,
    dropped: AtomicU64,
    deduped: AtomicU64,
    dedup_cache: SyncCache<DedupKey, ()>,
}

impl MpscEventSink {
    pub fn new(tx: mpsc::Sender<ExposureEvent>) -> Self {
        Self::with_dedup_config(tx, DedupConfig::default())
    }

    pub fn with_dedup_config(tx: mpsc::Sender<ExposureEvent>, config: DedupConfig) -> Self {
        let dedup_cache = SyncCache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.ttl)
            .build();
        Self {
            tx,
            dropped: AtomicU64::new(0),
            deduped: AtomicU64::new(0),
            dedup_cache,
        }
    }
}

impl EventSink for MpscEventSink {
    fn record_exposure(&self, event: ExposureEvent) {
        let key: DedupKey = (
            event.company_id.clone(),
            event.experiment_id.clone(),
            event.entity_id.clone(),
        );

        // Atomic insert-if-absent. `is_fresh()` is true only on the call
        // that actually populated the slot; concurrent callers for the
        // same (company, experiment, entity) tuple see `false` and bail.
        // The previous contains_key/insert pair was racy: under load, two
        // requests could both observe "absent" and both write to DuckDB.
        //
        // Insert happens before send-on-the-wire: if the channel is
        // saturated we still mark the tuple as deduped so a struggling
        // writer doesn't get a thundering herd of retries. The event is
        // lost, the `dropped` counter rises, and the next exposure for
        // the tuple has to wait out the TTL.
        let entry = self.dedup_cache.entry(key).or_insert_with(|| ());
        if !entry.is_fresh() {
            self.deduped.fetch_add(1, Ordering::Relaxed);
            return;
        }

        if self.tx.try_send(event).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    fn deduped(&self) -> u64 {
        self.deduped.load(Ordering::Relaxed)
    }
}

/// Sink that throws everything away. Used by tests that don't care about
/// exposure output.
pub struct NoopEventSink;

impl EventSink for NoopEventSink {
    fn record_exposure(&self, _event: ExposureEvent) {}
    fn dropped(&self) -> u64 {
        0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WriterConfig {
    pub batch_capacity: usize,
    pub flush_interval: Duration,
}

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS exposures (
    schema_version  INTEGER NOT NULL,
    ts_ms           BIGINT  NOT NULL,
    company_id      VARCHAR NOT NULL,
    experiment_id   VARCHAR NOT NULL,
    variant_key     VARCHAR,
    entity_id       VARCHAR NOT NULL
);
";

/// Open the DuckDB file and ensure the `exposures` table exists.
///
/// Run this from `main` before `spawn_writer` so a bad file path or a
/// schema migration mismatch fails the process at boot rather than silently
/// landing on the background writer task — where a failure would only show
/// up as exposures going missing.
pub fn bootstrap_duckdb_schema(duckdb_path: &Path) -> duckdb::Result<()> {
    let conn = Connection::open(duckdb_path)?;
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(())
}

pub fn spawn_writer(
    rx: mpsc::Receiver<ExposureEvent>,
    duckdb_path: PathBuf,
    config: WriterConfig,
) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || run_writer(rx, duckdb_path, config))
}

/// Periodically logs exposure-sink dedup/drop stats. Holds a `Weak` so it
/// exits naturally once the sink is dropped at shutdown.
pub fn spawn_sink_stats(sink: &Arc<dyn EventSink>, interval: Duration) {
    let weak = Arc::downgrade(sink);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut last_dropped = 0u64;
        let mut last_deduped = 0u64;
        loop {
            ticker.tick().await;
            let Some(sink) = weak.upgrade() else { return };
            let dropped = sink.dropped();
            let deduped = sink.deduped();
            drop(sink);

            let dropped_delta = dropped.saturating_sub(last_dropped);
            let deduped_delta = deduped.saturating_sub(last_deduped);
            last_dropped = dropped;
            last_deduped = deduped;

            if dropped_delta > 0 {
                tracing::warn!(
                    dropped_total = dropped,
                    dropped_delta,
                    deduped_total = deduped,
                    deduped_delta,
                    "exposure sink dropped events (channel saturated)",
                );
            } else if deduped_delta > 0 {
                tracing::info!(
                    deduped_total = deduped,
                    deduped_delta,
                    "exposure sink stats",
                );
            }
        }
    });
}

fn run_writer(
    mut rx: mpsc::Receiver<ExposureEvent>,
    duckdb_path: PathBuf,
    config: WriterConfig,
) {
    let conn = match Connection::open(&duckdb_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(
                path = %duckdb_path.display(),
                error = %e,
                "exposure: failed to open DuckDB",
            );
            drain_until_closed(&mut rx);
            return;
        }
    };

    // `memory_limit` is a per-connection PRAGMA, so it has to live here
    // rather than in `bootstrap_duckdb_schema`.
    if let Err(e) = conn.execute_batch("SET memory_limit='256MB'") {
        tracing::warn!(error = %e, "exposure: failed to set DuckDB memory_limit");
    }

    let runtime = tokio::runtime::Handle::current();
    let mut buf: Vec<ExposureEvent> = Vec::with_capacity(config.batch_capacity);
    let mut last_flush = Instant::now();

    loop {
        let elapsed = last_flush.elapsed();
        let should_flush = buf.len() >= config.batch_capacity
            || (!buf.is_empty() && elapsed >= config.flush_interval);

        if should_flush {
            let n = buf.len();
            match flush_batch(&conn, &mut buf) {
                Ok(()) => {
                    tracing::debug!(events = n, "exposure: flushed batch");
                }
                Err(e) => {
                    tracing::error!(
                        events = n,
                        error = %e,
                        "exposure: DuckDB flush failed, dropping events",
                    );
                    buf.clear();
                }
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
                let remaining = buf.len();
                let mut flushed = 0;
                if remaining > 0 {
                    match flush_batch(&conn, &mut buf) {
                        Ok(()) => flushed = remaining,
                        Err(e) => {
                            tracing::error!(
                                events = remaining,
                                error = %e,
                                "exposure: final DuckDB flush failed, dropping events",
                            );
                        }
                    }
                }
                tracing::info!(flushed, "exposure writer exiting");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::EXPOSURE_SCHEMA_VERSION;

    fn make_event(company: &str, exp: &str, entity: &str) -> ExposureEvent {
        ExposureEvent {
            schema_version: EXPOSURE_SCHEMA_VERSION,
            ts_ms: 0,
            company_id: company.to_string(),
            experiment_id: exp.to_string(),
            variant_key: Some("a".to_string()),
            entity_id: entity.to_string(),
        }
    }

    fn make_sink(capacity: usize) -> (MpscEventSink, mpsc::Receiver<ExposureEvent>) {
        let (tx, rx) = mpsc::channel(capacity);
        (MpscEventSink::new(tx), rx)
    }

    #[test]
    fn dedups_repeat_calls_for_same_tuple() {
        let (sink, mut rx) = make_sink(100);
        sink.record_exposure(make_event("co1", "exp1", "user1"));
        sink.record_exposure(make_event("co1", "exp1", "user1"));
        sink.record_exposure(make_event("co1", "exp1", "user1"));

        assert!(rx.try_recv().is_ok(), "first call should land on channel");
        assert!(rx.try_recv().is_err(), "subsequent calls must not enqueue");
        assert_eq!(sink.deduped(), 2);
        assert_eq!(sink.dropped(), 0);
    }

    #[test]
    fn does_not_dedup_across_entities() {
        let (sink, mut rx) = make_sink(100);
        sink.record_exposure(make_event("co1", "exp1", "user1"));
        sink.record_exposure(make_event("co1", "exp1", "user2"));

        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_ok());
        assert_eq!(sink.deduped(), 0);
    }

    #[test]
    fn does_not_dedup_across_experiments() {
        let (sink, mut rx) = make_sink(100);
        sink.record_exposure(make_event("co1", "exp1", "user1"));
        sink.record_exposure(make_event("co1", "exp2", "user1"));

        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_ok());
        assert_eq!(sink.deduped(), 0);
    }

    #[test]
    fn does_not_dedup_across_companies() {
        // Multi-tenant isolation: same entity_id under two companies must
        // produce two exposures, otherwise customer A's traffic could
        // suppress customer B's analytics.
        let (sink, mut rx) = make_sink(100);
        sink.record_exposure(make_event("co1", "exp1", "user1"));
        sink.record_exposure(make_event("co2", "exp1", "user1"));

        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_ok());
        assert_eq!(sink.deduped(), 0);
    }

    #[test]
    fn channel_full_increments_dropped_and_subsequent_calls_dedup() {
        // Capacity 1: one event lands, second different-tuple event hits
        // channel-full and increments `dropped`. A third call with that
        // same second tuple must be deduped (not retried), so we don't
        // pile up dropped events on a struggling writer.
        let (sink, mut rx) = make_sink(1);
        sink.record_exposure(make_event("co1", "exp1", "user1"));
        sink.record_exposure(make_event("co1", "exp1", "user2"));
        sink.record_exposure(make_event("co1", "exp1", "user2"));

        assert_eq!(sink.dropped(), 1);
        assert_eq!(sink.deduped(), 1);
        // First event drained.
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn dedup_with_zero_capacity_disables_caching() {
        // A 0-capacity dedup cache short-circuits to "always insert" — used
        // here as a regression guard so a misconfigured deploy still records
        // exposures (it just records duplicates, which is the old behavior).
        let (tx, mut rx) = mpsc::channel(100);
        let sink = MpscEventSink::with_dedup_config(
            tx,
            DedupConfig { max_capacity: 0, ttl: Duration::from_secs(60) },
        );
        sink.record_exposure(make_event("co1", "exp1", "user1"));
        sink.record_exposure(make_event("co1", "exp1", "user1"));

        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_ok(), "0-capacity cache should not dedup");
    }
}
