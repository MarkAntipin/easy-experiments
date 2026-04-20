use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::enums::ExperimentStatus;
use crate::models::CreateExperimentRequest;
use sqlx::sqlite::SqlitePool;

pub struct ExperimentsDB {
    pub pool: SqlitePool,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ExperimentDBRow {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub status: ExperimentStatus,
    pub variants: String,

    #[serde(rename = "trafficPercentage")]
    pub traffic_percentage: f64,

    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

impl From<CreateExperimentRequest> for ExperimentDBRow {
    fn from(request: CreateExperimentRequest) -> Self {
        let now = Utc::now();
        let variants_json = serde_json::to_string(&request.variants).unwrap_or_else(|_| "[]".to_string());
        Self {
            id: 0,
            name: request.name,
            description: request.description.unwrap_or_default(),
            status: ExperimentStatus::Draft,
            variants: variants_json,
            traffic_percentage: request.traffic_percentage.unwrap_or(100.0),
            created_at: now,
            updated_at: now,
        }
    }
}
