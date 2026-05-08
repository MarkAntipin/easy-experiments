use serde::{Deserialize, Serialize};

use crate::errors::CustomError;
use crate::validation::Validate;

const MAX_ENTITY_ID_LENGTH: usize = 256;
const MAX_METRIC_NAME_LENGTH: usize = 256;
const MAX_IDEMPOTENCY_KEY_LENGTH: usize = 128;
pub const MAX_TRACK_EVENTS_PER_REQUEST: usize = 100;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackEvent {
    pub entity_id: String,
    pub metric_name: String,
    /// `None` = binary count event (treated as 1.0 in aggregation).
    #[serde(default)]
    pub value: Option<f64>,
    /// Server stamps `Utc::now()` if omitted.
    #[serde(default)]
    pub ts: Option<i64>,
    /// Optional client-supplied dedup key. When present, repeats with the
    /// same `(company_id, idempotency_key)` are dropped at the sink.
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackRequest {
    pub events: Vec<TrackEvent>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackResponse {
    /// Count of events accepted into the pipeline. Excludes events dropped by
    /// the idempotency cache.
    pub accepted: usize,
    /// Count suppressed by sink-level idempotency.
    pub deduped: usize,
}

impl Validate for TrackRequest {
    fn validate(&self) -> Result<(), CustomError> {
        if self.events.is_empty() {
            return Err(CustomError::ValidationError(
                "events must not be empty".into(),
            ));
        }
        if self.events.len() > MAX_TRACK_EVENTS_PER_REQUEST {
            return Err(CustomError::ValidationError(format!(
                "events must contain at most {} entries",
                MAX_TRACK_EVENTS_PER_REQUEST
            )));
        }
        for ev in &self.events {
            if ev.entity_id.is_empty() {
                return Err(CustomError::ValidationError(
                    "entityId must not be empty".into(),
                ));
            }
            if ev.entity_id.len() > MAX_ENTITY_ID_LENGTH {
                return Err(CustomError::ValidationError(format!(
                    "entityId length should be less than {} bytes",
                    MAX_ENTITY_ID_LENGTH
                )));
            }
            if ev.metric_name.is_empty() {
                return Err(CustomError::ValidationError(
                    "metricName must not be empty".into(),
                ));
            }
            if ev.metric_name.len() > MAX_METRIC_NAME_LENGTH {
                return Err(CustomError::ValidationError(format!(
                    "metricName length should be less than {} bytes",
                    MAX_METRIC_NAME_LENGTH
                )));
            }
            if let Some(v) = ev.value {
                if !v.is_finite() {
                    return Err(CustomError::ValidationError(
                        "value must be a finite number".into(),
                    ));
                }
            }
            if let Some(key) = ev.idempotency_key.as_deref() {
                if key.is_empty() {
                    return Err(CustomError::ValidationError(
                        "idempotencyKey must not be empty when provided".into(),
                    ));
                }
                if key.len() > MAX_IDEMPOTENCY_KEY_LENGTH {
                    return Err(CustomError::ValidationError(format!(
                        "idempotencyKey length should be less than {} bytes",
                        MAX_IDEMPOTENCY_KEY_LENGTH
                    )));
                }
            }
        }
        Ok(())
    }
}
