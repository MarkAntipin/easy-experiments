pub const METRIC_SCHEMA_VERSION: u16 = 1;

/// One metric/conversion event recorded against an entity. Not tied to a
/// specific experiment at write time — attribution happens at query time by
/// joining `metric_events` to `exposures` on `(company_id, entity_id)`.
///
/// `metric_value`:
/// - `None` for binary count metrics (e.g. "signed_up"); aggregation treats
///   each row as 1.0.
/// - `Some(v)` for continuous metrics (e.g. "purchase_revenue").
#[derive(Debug, Clone)]
pub struct MetricEvent {
    pub schema_version: u16,
    pub ts_ms: i64,
    pub company_id: String,
    pub entity_id: String,
    pub metric_name: String,
    pub metric_value: Option<f64>,
}

impl MetricEvent {
    pub fn new(
        ts_ms: i64,
        company_id: String,
        entity_id: String,
        metric_name: String,
        metric_value: Option<f64>,
    ) -> Self {
        Self {
            schema_version: METRIC_SCHEMA_VERSION,
            ts_ms,
            company_id,
            entity_id,
            metric_name,
            metric_value,
        }
    }
}
