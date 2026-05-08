use config::{Config as RawConfig, Environment};
use dotenv::dotenv;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub application_port: u16,

    pub jwt_secret: Option<String>,

    pub google_client_id: Option<String>,

    pub google_jwks_url: String,

    pub sqlite_url: String,

    pub duckdb_path: String,

    pub event_queue_capacity: usize,

    pub event_batch_capacity: usize,

    pub event_flush_interval_ms: u64,

    pub exposure_dedup_capacity: u64,

    pub exposure_dedup_ttl_secs: u64,

    pub metric_queue_capacity: usize,

    pub metric_batch_capacity: usize,

    pub metric_flush_interval_ms: u64,

    pub metric_idempotency_capacity: u64,

    pub metric_idempotency_ttl_secs: u64,

    pub analytics_pool_size: usize,

    pub analytics_cache_capacity: u64,

    pub analytics_cache_ttl_secs: u64,

    pub cors_allowed_origins: Option<String>,
}

impl Config {
    pub fn sqlite_filepath(&self) -> PathBuf {
        let path = self
            .sqlite_url
            .strip_prefix("sqlite://")
            .unwrap_or(&self.sqlite_url);
        PathBuf::from(path)
    }
}

pub fn get_config() -> Result<Config, config::ConfigError> {
    dotenv().ok();

    RawConfig::builder()
        .set_default("application_port", 18200)?
        .set_default("sqlite_url", "sqlite://easy-experiments.db")?
        .set_default("duckdb_path", "easy-experiments.duckdb")?
        .set_default("event_queue_capacity", 10_000)?
        .set_default("event_batch_capacity", 1_000)?
        .set_default("event_flush_interval_ms", 1_000)?
        .set_default("exposure_dedup_capacity", 1_000_000)?
        .set_default("exposure_dedup_ttl_secs", 3_600)?
        .set_default("metric_queue_capacity", 10_000)?
        .set_default("metric_batch_capacity", 1_000)?
        .set_default("metric_flush_interval_ms", 1_000)?
        .set_default("metric_idempotency_capacity", 200_000)?
        .set_default("metric_idempotency_ttl_secs", 600)?
        .set_default("analytics_pool_size", 4)?
        .set_default("analytics_cache_capacity", 1_000)?
        .set_default("analytics_cache_ttl_secs", 30)?
        .set_default(
            "google_jwks_url",
            crate::services::google_auth::DEFAULT_GOOGLE_JWKS_URL,
        )?
        .add_source(Environment::default().separator(""))
        .build()?
        .try_deserialize()
}
