use serde::{Deserialize, Deserializer, Serialize};

use crate::errors::CustomError;
use crate::validation::Validate;

use super::domain::{Segment, Variant};
use super::status::ExperimentStatus;
use super::validation::{validate_experiment_state, validate_key};

fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateExperimentRequest {
    pub key: String,
    #[serde(default)]
    pub description: Option<String>,
    pub primary_metric: String,
    pub segments: Vec<Segment>,
    pub variants: Vec<Variant>,
}

impl Validate for CreateExperimentRequest {
    fn validate(&self) -> Result<(), CustomError> {
        validate_key(&self.key)?;
        validate_experiment_state(
            self.description.as_deref(),
            &self.primary_metric,
            &self.variants,
            &self.segments,
        )
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateExperimentRequest {
    #[serde(default, deserialize_with = "double_option")]
    pub description: Option<Option<String>>,
    pub primary_metric: Option<String>,
    pub segments: Option<Vec<Segment>>,
    pub variants: Option<Vec<Variant>>,
}

impl Validate for UpdateExperimentRequest {
    fn validate(&self) -> Result<(), CustomError> {
        if self.description.is_none()
            && self.primary_metric.is_none()
            && self.segments.is_none()
            && self.variants.is_none()
        {
            return Err(CustomError::ValidationError(
                "Request body must include at least one field to update".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct GetExperimentsQueryParams {
    #[serde(default)]
    pub status: Option<ExperimentStatus>,
}
