use serde::{Deserialize, Serialize};

use crate::errors::CustomError;

const MAX_NAME_LENGTH: usize = 256;
const MAX_DESCRIPTION_LENGTH: usize = 4096;

#[derive(Serialize, Deserialize, Clone)]
pub struct Variant {
    pub name: String,
    pub weight: f64,
}

#[derive(Serialize, Deserialize)]
pub struct CreateExperimentRequest {
    pub name: String,
    pub description: Option<String>,
    pub variants: Vec<Variant>,
    #[serde(rename = "trafficPercentage")]
    pub traffic_percentage: Option<f64>,
}

impl CreateExperimentRequest {
    pub fn validate(&self) -> Result<(), CustomError> {
        if self.name.is_empty() {
            return Err(CustomError::ValidationError("Name should not be empty".to_string()));
        }
        if self.name.len() > MAX_NAME_LENGTH {
            return Err(CustomError::ValidationError(format!(
                "Name length should be less than {} bytes", MAX_NAME_LENGTH
            )));
        }
        if let Some(ref desc) = self.description {
            if desc.len() > MAX_DESCRIPTION_LENGTH {
                return Err(CustomError::ValidationError(format!(
                    "Description length should be less than {} bytes", MAX_DESCRIPTION_LENGTH
                )));
            }
        }
        if self.variants.is_empty() {
            return Err(CustomError::ValidationError("Variants should not be empty".to_string()));
        }
        for variant in &self.variants {
            if variant.name.is_empty() {
                return Err(CustomError::ValidationError("Variant name should not be empty".to_string()));
            }
            if variant.weight < 0.0 {
                return Err(CustomError::ValidationError("Variant weight should not be negative".to_string()));
            }
        }
        if let Some(tp) = self.traffic_percentage {
            if !(0.0..=100.0).contains(&tp) {
                return Err(CustomError::ValidationError(
                    "Traffic percentage should be between 0 and 100".to_string()
                ));
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct UpdateExperimentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub variants: Option<Vec<Variant>>,
    #[serde(rename = "trafficPercentage")]
    pub traffic_percentage: Option<f64>,
}

impl UpdateExperimentRequest {
    pub fn validate(&self) -> Result<(), CustomError> {
        if let Some(ref name) = self.name {
            if name.is_empty() {
                return Err(CustomError::ValidationError("Name should not be empty".to_string()));
            }
            if name.len() > MAX_NAME_LENGTH {
                return Err(CustomError::ValidationError(format!(
                    "Name length should be less than {} bytes", MAX_NAME_LENGTH
                )));
            }
        }
        if let Some(ref desc) = self.description {
            if desc.len() > MAX_DESCRIPTION_LENGTH {
                return Err(CustomError::ValidationError(format!(
                    "Description length should be less than {} bytes", MAX_DESCRIPTION_LENGTH
                )));
            }
        }
        if let Some(ref variants) = self.variants {
            if variants.is_empty() {
                return Err(CustomError::ValidationError("Variants should not be empty".to_string()));
            }
            for variant in variants {
                if variant.name.is_empty() {
                    return Err(CustomError::ValidationError("Variant name should not be empty".to_string()));
                }
                if variant.weight < 0.0 {
                    return Err(CustomError::ValidationError("Variant weight should not be negative".to_string()));
                }
            }
        }
        if let Some(tp) = self.traffic_percentage {
            if !(0.0..=100.0).contains(&tp) {
                return Err(CustomError::ValidationError(
                    "Traffic percentage should be between 0 and 100".to_string()
                ));
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct ValidateTokenRequest {
    pub token: String,
}

#[derive(Serialize, Deserialize)]
pub struct EvaluateRequest {
    #[serde(rename = "experimentId")]
    pub experiment_id: i64,
}

impl EvaluateRequest {
    pub fn validate(&self) -> Result<(), CustomError> {
        if self.experiment_id <= 0 {
            return Err(CustomError::ValidationError("Experiment ID should be positive".to_string()));
        }
        Ok(())
    }
}
