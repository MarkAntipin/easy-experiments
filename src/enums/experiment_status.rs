use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum ExperimentStatus {
    Draft,
    Active,
    Paused,
    Completed,
}

impl fmt::Display for ExperimentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExperimentStatus::Draft => write!(f, "draft"),
            ExperimentStatus::Active => write!(f, "active"),
            ExperimentStatus::Paused => write!(f, "paused"),
            ExperimentStatus::Completed => write!(f, "completed"),
        }
    }
}

impl From<String> for ExperimentStatus {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "draft" => ExperimentStatus::Draft,
            "active" => ExperimentStatus::Active,
            "paused" => ExperimentStatus::Paused,
            "completed" => ExperimentStatus::Completed,
            _ => ExperimentStatus::Draft,
        }
    }
}
