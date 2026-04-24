use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

use super::ExperimentStatus;

pub struct ExperimentsDB {
    pub pool: SqlitePool,
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

#[derive(Serialize, Deserialize, sqlx::FromRow)]
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

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Variant {
    pub key: String,
    pub is_control: bool,
    #[serde(default = "default_attachment")]
    pub attachment: serde_json::Value,
}

fn default_attachment() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub rank: i32,
    pub rollout_percent: u32,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    pub distributions: Vec<Distribution>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConstraintOperator {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    In,
    NotIn,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Constraint {
    pub property: String,
    pub operator: ConstraintOperator,
    pub value: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Distribution {
    pub variant_key: String,
    pub percent: u32,
}
