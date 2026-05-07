use std::time::Duration;

use moka::future::Cache;
use sqlx::sqlite::SqlitePool;

use super::api_key::ApiKeyCache;
use super::experiment::ExperimentCache;

pub struct ExperimentsDB {
    pub pool: SqlitePool,
    pub experiment_cache: ExperimentCache,
    pub api_key_cache: ApiKeyCache,
}

impl ExperimentsDB {
    pub fn new(pool: SqlitePool) -> Self {
        let experiment_cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(300))
            .build();
        // Short TTL because revoking an API key needs to take effect quickly.
        // 60s is the worst-case staleness window for a revoked key.
        let api_key_cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(60))
            .build();
        Self { pool, experiment_cache, api_key_cache }
    }
}
