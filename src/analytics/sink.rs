use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::mpsc;

use super::event::ExposureEvent;

/// Abstraction for "where do exposure events go".
///
/// Today's only real impl is `MpscEventSink`, which hands events to a local
/// DuckDB writer task. Tomorrow's impl could be a Kafka producer, an S3
/// Parquet batcher, etc. Handlers depend only on this trait.
pub trait EventSink: Send + Sync + 'static {
    fn record_exposure(&self, event: ExposureEvent);
    fn dropped(&self) -> u64;
}

/// In-process bounded mpsc sink. Non-blocking on `record_exposure`: if the
/// channel is full, the event is dropped and a counter is incremented.
pub struct MpscEventSink {
    tx: mpsc::Sender<ExposureEvent>,
    dropped: AtomicU64,
}

impl MpscEventSink {
    pub fn new(tx: mpsc::Sender<ExposureEvent>) -> Self {
        Self { tx, dropped: AtomicU64::new(0) }
    }
}

impl EventSink for MpscEventSink {
    fn record_exposure(&self, event: ExposureEvent) {
        if self.tx.try_send(event).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

/// Sink that throws everything away. Used by tests that don't care about
/// analytics output.
pub struct NoopEventSink;

impl EventSink for NoopEventSink {
    fn record_exposure(&self, _event: ExposureEvent) {}
    fn dropped(&self) -> u64 {
        0
    }
}
