use std::collections::HashMap;
use std::sync::Arc;

use moka::future::Cache;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::domain::Segment;
use super::status::ExperimentStatus;

pub type ExperimentCacheKey = (String, String);
/// Cache value is `Option<Arc<...>>` so misses are memoized too. The repository
/// only loads `status = 'running'` rows, so a `None` covers both "doesn't
/// exist" and "exists but not running" — a tenant hammering `/evaluate` for
/// unknown or paused keys still costs at most one SQLite query per TTL.
pub type ExperimentCache = Cache<ExperimentCacheKey, Option<Arc<CachedExperiment>>>;

/// Parsed, evaluation-ready view of a running experiment.
///
/// Stored in the cache so the hot path doesn't re-parse the variants/segments
/// JSON on every evaluate. Segments are pre-sorted by `priority` ascending.
/// `variant_configs` is keyed for O(1) config lookup by variant key, and the
/// config is `Arc`-shared so the evaluate path returns it without cloning the
/// underlying JSON. Status is implicit (always `Running`) — non-running rows
/// are filtered out at load time.
pub struct CachedExperiment {
    pub experiment_id: String,
    pub variant_configs: HashMap<String, Arc<Value>>,
    pub segments: Vec<Segment>,
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

#[derive(sqlx::FromRow)]
pub struct ExperimentListRow {
    pub experiment_id: String,
    pub key: String,
    pub description: Option<String>,
    pub status: ExperimentStatus,
    pub primary_metric: String,
    pub started_at: Option<i64>,
    pub stopped_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}
