use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::sqlite::SqlitePool;

use super::{ExperimentStatus, Segment};

pub type ExperimentCacheKey = (String, String);
pub type ExperimentCache = Cache<ExperimentCacheKey, Arc<CachedExperiment>>;

pub type ApiKeyCache = Cache<String, Arc<ApiKeyRow>>;

/// Parsed, evaluation-ready view of an experiment.
///
/// Stored in the cache so the hot path doesn't re-parse the variants/segments
/// JSON on every evaluate. Segments are pre-sorted by `priority` ascending.
/// `variant_configs` is keyed for O(1) config lookup by variant key, and the
/// config is `Arc`-shared so the evaluate path returns it without cloning the
/// underlying JSON.
pub struct CachedExperiment {
    pub experiment_id: String,
    pub status: ExperimentStatus,
    pub variant_configs: HashMap<String, Arc<Value>>,
    pub segments: Vec<Segment>,
}

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

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct CompanyRow {
    pub company_id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct UserRow {
    pub user_id: String,
    pub company_id: String,
    pub email: String,
    pub name: Option<String>,
    pub picture_url: Option<String>,
    pub google_sub: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiKeyRow {
    pub api_key_id: String,
    pub company_id: String,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct ExperimentRow {
    pub experiment_id: String,
    pub key: String,
    pub description: Option<String>,
    pub status: ExperimentStatus,
    pub primary_metric: String,
    pub variants: String,
    pub segments: String,
    pub started_at: Option<i64>,
    pub stopped_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub company_id: String,
}
