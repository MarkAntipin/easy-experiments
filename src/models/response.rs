use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::errors::CustomError;
use crate::models::db::{ApiKeyRow, CompanyRow, ExperimentRow, UserRow};
use crate::models::domain::{Segment, Variant};

#[derive(Serialize, Deserialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateExperimentResponse {
    pub experiment_id: String,
    pub message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentResponse {
    pub experiment_id: String,
    pub key: String,
    pub description: Option<String>,
    pub status: String,
    pub primary_metric: String,
    pub variants: Vec<Variant>,
    pub segments: Vec<Segment>,
    pub started_at: Option<i64>,
    pub stopped_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ExperimentResponse {
    pub fn from_row(experiment: ExperimentRow) -> Result<Self, CustomError> {
        let variants: Vec<Variant> = serde_json::from_str(&experiment.variants).map_err(|e| {
            CustomError::InternalError(format!("Failed to parse stored variants: {}", e))
        })?;
        let segments: Vec<Segment> = serde_json::from_str(&experiment.segments).map_err(|e| {
            CustomError::InternalError(format!("Failed to parse stored segments: {}", e))
        })?;

        Ok(Self {
            experiment_id: experiment.experiment_id,
            key: experiment.key,
            description: experiment.description,
            status: experiment.status.to_string(),
            primary_metric: experiment.primary_metric,
            variants,
            segments,
            started_at: experiment.started_at,
            stopped_at: experiment.stopped_at,
            created_at: experiment.created_at,
            updated_at: experiment.updated_at,
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentListItem {
    pub experiment_id: String,
    pub key: String,
    pub description: Option<String>,
    pub status: String,
    pub primary_metric: String,
    pub started_at: Option<i64>,
    pub stopped_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<ExperimentRow> for ExperimentListItem {
    fn from(row: ExperimentRow) -> Self {
        Self {
            experiment_id: row.experiment_id,
            key: row.key,
            description: row.description,
            status: row.status.to_string(),
            primary_metric: row.primary_metric,
            started_at: row.started_at,
            stopped_at: row.stopped_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateResponse {
    pub experiment_key: String,
    pub variant_key: Option<String>,
    pub config: Option<Arc<serde_json::Value>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
    pub company: CompanyResponse,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub user_id: String,
    pub email: String,
    pub name: Option<String>,
    pub picture_url: Option<String>,
}

impl From<UserRow> for UserResponse {
    fn from(row: UserRow) -> Self {
        Self {
            user_id: row.user_id,
            email: row.email,
            name: row.name,
            picture_url: row.picture_url,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyResponse {
    pub company_id: String,
    pub name: String,
}

impl From<CompanyRow> for CompanyResponse {
    fn from(row: CompanyRow) -> Self {
        Self {
            company_id: row.company_id,
            name: row.name,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyResponse {
    pub api_key_id: String,
    pub name: String,
    pub key: String,
    pub key_prefix: String,
    pub created_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyListItem {
    pub api_key_id: String,
    pub name: String,
    pub key_prefix: String,
    pub created_at: i64,
}

impl From<ApiKeyRow> for ApiKeyListItem {
    fn from(row: ApiKeyRow) -> Self {
        Self {
            api_key_id: row.api_key_id,
            name: row.name,
            key_prefix: row.key_prefix,
            created_at: row.created_at,
        }
    }
}
