use std::collections::HashMap;
use std::sync::Arc;

use moka::future::Cache;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::domain::Segment;
use super::status::ExperimentStatus;

pub type ExperimentCacheKey = (String, String);
pub type ExperimentCache = Cache<ExperimentCacheKey, Arc<CachedExperiment>>;

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
