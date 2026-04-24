use serde::Deserialize;

use crate::models::ExperimentStatus;

#[derive(Deserialize)]
pub struct GetExperimentsQueryParams {
    #[serde(default)]
    pub status: Option<ExperimentStatus>,
}
