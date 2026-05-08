pub const EXPOSURE_SCHEMA_VERSION: u16 = 1;

// TODO: What do I need for plot display
#[derive(Debug, Clone)]
pub struct ExposureEvent {
    pub schema_version: u16,
    pub ts_ms: i64,
    pub company_id: String,
    pub experiment_id: String,
    pub variant_key: Option<String>,
    pub entity_id: String,
}

impl ExposureEvent {
    pub fn new(
        ts_ms: i64,
        company_id: String,
        experiment_id: String,
        variant_key: Option<String>,
        entity_id: String,
    ) -> Self {
        Self {
            schema_version: EXPOSURE_SCHEMA_VERSION,
            ts_ms,
            company_id,
            experiment_id,
            variant_key,
            entity_id,
        }
    }
}
