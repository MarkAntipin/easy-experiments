use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use duckdb::{params, Connection};
use moka::sync::Cache as SyncCache;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::models::MetricEvent;

/// Where metric events go. Mirrors `EventSink` for exposures but keyed by a
/// different event type and dedup discipline (idempotency-key based, not
/// (company, experiment, entity) tuple based).
pub trait MetricSink: Send + Sync + 'static {
    fn record(&self, event: MetricEvent, idempotency_key: Option<&str>) -> RecordOutcome;
    fn dropped(&self) -> u64;
    fn deduped(&self) -> u64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordOutcome {
    Accepted,
    Deduped,
    Dropped,
}

#[derive(Debug, Clone, Copy)]
pub struct MetricDedupConfig {
    pub max_capacity: u64,
    pub ttl: Duration,
}

impl Default for MetricDedupConfig {
    fn default() -> Self {
        Self {
            max_capacity: 200_000,
            // 10 minutes covers retry windows for typical SDK back-off without
            // suppressing legitimate repeats from the same caller hours later.
            ttl: Duration::from_secs(10 * 60),
        }
    }
}

type DedupKey = (String, String);

/// In-process bounded mpsc sink with optional idempotency-key dedup.
///
/// Unlike exposures, repeat metric events for the same (company, entity,
/// metric) are *legitimate* (think: multiple purchases). So we only dedup
/// when the client supplied an `idempotency_key`, and only against an exact
/// `(company_id, idempotency_key)` match.
pub struct MpscMetricSink {
    tx: mpsc::Sender<MetricEvent>,
    dropped: AtomicU64,
    deduped: AtomicU64,
    dedup_cache: SyncCache<DedupKey, ()>,
}

impl MpscMetricSink {
    pub fn new(tx: mpsc::Sender<MetricEvent>) -> Self {
        Self::with_dedup_config(tx, MetricDedupConfig::default())
    }

    pub fn with_dedup_config(tx: mpsc::Sender<MetricEvent>, config: MetricDedupConfig) -> Self {
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

impl MetricSink for MpscMetricSink {
    fn record(&self, event: MetricEvent, idempotency_key: Option<&str>) -> RecordOutcome {
        if let Some(key) = idempotency_key {
            let cache_key = (event.company_id.clone(), key.to_string());
            // Atomic insert-if-absent. Concurrent callers with the same key
            // see is_fresh() == false and bail.
            let entry = self.dedup_cache.entry(cache_key).or_insert_with(|| ());
            if !entry.is_fresh() {
                self.deduped.fetch_add(1, Ordering::Relaxed);
                return RecordOutcome::Deduped;
            }
        }

        if self.tx.try_send(event).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return RecordOutcome::Dropped;
        }
        RecordOutcome::Accepted
    }

    fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    fn deduped(&self) -> u64 {
        self.deduped.load(Ordering::Relaxed)
    }
}

/// Sink that throws every metric away. Used by tests that don't want a
/// DuckDB file backing the metric pipeline.
pub struct NoopMetricSink;

impl MetricSink for NoopMetricSink {
    fn record(&self, _event: MetricEvent, _idempotency_key: Option<&str>) -> RecordOutcome {
        RecordOutcome::Accepted
    }
    fn dropped(&self) -> u64 {
        0
    }
    fn deduped(&self) -> u64 {
        0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MetricWriterConfig {
    pub batch_capacity: usize,
    pub flush_interval: Duration,
}

pub fn spawn_metric_writer(
    rx: mpsc::Receiver<MetricEvent>,
    conn: Connection,
    config: MetricWriterConfig,
) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || run_metric_writer(rx, conn, config))
}

fn run_metric_writer(
    mut rx: mpsc::Receiver<MetricEvent>,
    conn: Connection,
    config: MetricWriterConfig,
) {
    if let Err(e) = conn.execute_batch("SET memory_limit='256MB'") {
        tracing::warn!(error = %e, "metric: failed to set DuckDB memory_limit");
    }

    let runtime = tokio::runtime::Handle::current();
    let mut buf: Vec<MetricEvent> = Vec::with_capacity(config.batch_capacity);
    let mut last_flush = Instant::now();

    loop {
        let elapsed = last_flush.elapsed();
        let should_flush = buf.len() >= config.batch_capacity
            || (!buf.is_empty() && elapsed >= config.flush_interval);

        if should_flush {
            let n = buf.len();
            match flush_batch(&conn, &mut buf) {
                Ok(()) => tracing::debug!(events = n, "metric: flushed batch"),
                Err(e) => {
                    tracing::error!(
                        events = n,
                        error = %e,
                        "metric: DuckDB flush failed, dropping events",
                    );
                    buf.clear();
                }
            }
            last_flush = Instant::now();
            continue;
        }

        let timeout = config.flush_interval.saturating_sub(elapsed);
        let want = config.batch_capacity.saturating_sub(buf.len()).max(1);

        let n = runtime
            .block_on(async { tokio::time::timeout(timeout, rx.recv_many(&mut buf, want)).await });

        match n {
            Ok(0) => {
                if !buf.is_empty() {
                    if let Err(e) = flush_batch(&conn, &mut buf) {
                        tracing::error!(
                            events = buf.len(),
                            error = %e,
                            "metric: final DuckDB flush failed, dropping events",
                        );
                    }
                }
                return;
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }
}

fn flush_batch(conn: &Connection, buf: &mut Vec<MetricEvent>) -> duckdb::Result<()> {
    let mut app = conn.appender("metric_events")?;
    for ev in buf.iter() {
        app.append_row(params![
            ev.schema_version as i32,
            ev.ts_ms,
            ev.company_id,
            ev.entity_id,
            ev.metric_name,
            ev.metric_value,
        ])?;
    }
    app.flush()?;
    buf.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::METRIC_SCHEMA_VERSION;

    fn make_event(company: &str, entity: &str, metric: &str) -> MetricEvent {
        MetricEvent {
            schema_version: METRIC_SCHEMA_VERSION,
            ts_ms: 0,
            company_id: company.to_string(),
            entity_id: entity.to_string(),
            metric_name: metric.to_string(),
            metric_value: None,
        }
    }

    fn make_sink(capacity: usize) -> (MpscMetricSink, mpsc::Receiver<MetricEvent>) {
        let (tx, rx) = mpsc::channel(capacity);
        (MpscMetricSink::new(tx), rx)
    }

    #[test]
    fn accepts_event_without_idempotency_key() {
        let (sink, mut rx) = make_sink(10);
        assert_eq!(
            sink.record(make_event("co", "u1", "m1"), None),
            RecordOutcome::Accepted
        );
        assert!(rx.try_recv().is_ok());
    }

    #[test]
    fn allows_repeat_events_when_no_idempotency_key() {
        let (sink, mut rx) = make_sink(10);
        sink.record(make_event("co", "u1", "m1"), None);
        sink.record(make_event("co", "u1", "m1"), None);
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_ok());
        assert_eq!(sink.deduped(), 0);
    }

    #[test]
    fn dedups_repeat_events_with_same_idempotency_key() {
        let (sink, mut rx) = make_sink(10);
        assert_eq!(
            sink.record(make_event("co", "u1", "m1"), Some("k1")),
            RecordOutcome::Accepted
        );
        assert_eq!(
            sink.record(make_event("co", "u1", "m1"), Some("k1")),
            RecordOutcome::Deduped
        );
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_err());
        assert_eq!(sink.deduped(), 1);
    }

    #[test]
    fn idempotency_keys_are_scoped_per_company() {
        // Two tenants using the same key value must not collide.
        let (sink, mut rx) = make_sink(10);
        sink.record(make_event("co1", "u1", "m1"), Some("k1"));
        sink.record(make_event("co2", "u1", "m1"), Some("k1"));
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_ok());
        assert_eq!(sink.deduped(), 0);
    }

    #[test]
    fn channel_full_increments_dropped() {
        let (sink, mut rx) = make_sink(1);
        assert_eq!(
            sink.record(make_event("co", "u1", "m1"), None),
            RecordOutcome::Accepted
        );
        assert_eq!(
            sink.record(make_event("co", "u2", "m1"), None),
            RecordOutcome::Dropped
        );
        assert_eq!(sink.dropped(), 1);
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_err());
    }
}
