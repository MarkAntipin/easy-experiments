use serde::{Deserialize, Serialize};

use crate::errors::CustomError;

use super::db::{ExperimentListRow, ExperimentRow};
use super::domain::{Segment, Variant};
use super::status::ExperimentStatus;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateExperimentResponse {
    pub experiment_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentResponse {
    pub experiment_id: String,
    pub key: String,
    pub description: Option<String>,
    pub status: ExperimentStatus,
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
            status: experiment.status,
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
    pub status: ExperimentStatus,
    pub primary_metric: String,
    pub started_at: Option<i64>,
    pub stopped_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<ExperimentListRow> for ExperimentListItem {
    fn from(row: ExperimentListRow) -> Self {
        Self {
            experiment_id: row.experiment_id,
            key: row.key,
            description: row.description,
            status: row.status,
            primary_metric: row.primary_metric,
            started_at: row.started_at,
            stopped_at: row.stopped_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Serialize)]
pub struct ExperimentListResponse {
    pub items: Vec<ExperimentListItem>,
}
