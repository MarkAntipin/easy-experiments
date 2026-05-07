use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum ExperimentStatus {
    Draft,
    Running,
    Stopped,
    Deleted,
}

impl fmt::Display for ExperimentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExperimentStatus::Draft => write!(f, "draft"),
            ExperimentStatus::Running => write!(f, "running"),
            ExperimentStatus::Stopped => write!(f, "stopped"),
            ExperimentStatus::Deleted => write!(f, "deleted"),
        }
    }
}
