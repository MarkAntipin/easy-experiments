use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::errors::CustomError;
use crate::models::experiment::MAX_KEY_LENGTH;
use crate::validation::Validate;

const MAX_ENTITY_ID_LENGTH: usize = 256;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateRequest {
    pub experiment_key: String,
    pub entity_id: String,
    #[serde(default)]
    pub properties: serde_json::Value,
}

impl Validate for EvaluateRequest {
    fn validate(&self) -> Result<(), CustomError> {
        if self.experiment_key.is_empty() {
            return Err(CustomError::ValidationError("Experiment key should not be empty".into()));
        }
        if self.experiment_key.len() > MAX_KEY_LENGTH {
            return Err(CustomError::ValidationError(format!(
                "Experiment key length should be less than {} bytes", MAX_KEY_LENGTH
            )));
        }
        if self.entity_id.is_empty() {
            return Err(CustomError::ValidationError("Entity ID should not be empty".into()));
        }
        if self.entity_id.len() > MAX_ENTITY_ID_LENGTH {
            return Err(CustomError::ValidationError(format!(
                "Entity ID length should be less than {} bytes", MAX_ENTITY_ID_LENGTH
            )));
        }
        // Constraints look up properties as `properties.get(name)`; that only
        // makes sense when properties is an object (or null = no properties).
        // Reject arrays/strings/etc here so non-object inputs don't silently
        // make NEQ/NOT_IN constraints pass for every property.
        if !self.properties.is_null() && !self.properties.is_object() {
            return Err(CustomError::ValidationError(
                "Properties must be a JSON object".into(),
            ));
        }
        // Total payload size is bounded by the per-route `JsonConfig::limit`
        // in `startup.rs`; no need to re-serialize here just to count bytes.
        Ok(())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateResponse {
    pub experiment_key: String,
    pub variant_key: Option<String>,
    pub config: Option<Arc<serde_json::Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn assert_validation_err(result: Result<(), CustomError>, needle: &str) {
        match result {
            Err(CustomError::ValidationError(msg)) => assert!(
                msg.contains(needle),
                "expected validation error containing {needle:?}, got {msg:?}"
            ),
            other => panic!("expected ValidationError({needle:?}), got {other:?}"),
        }
    }

    fn evaluate_request(properties: serde_json::Value) -> EvaluateRequest {
        EvaluateRequest {
            experiment_key: "exp".into(),
            entity_id: "user-1".into(),
            properties,
        }
    }

    #[test]
    fn evaluate_request_accepts_null_or_object_properties() {
        assert!(evaluate_request(json!(null)).validate().is_ok());
        assert!(evaluate_request(json!({})).validate().is_ok());
        assert!(evaluate_request(json!({"country": "US", "tier": 1})).validate().is_ok());
    }

    #[test]
    fn evaluate_request_rejects_non_object_properties() {
        for v in [json!("foo"), json!(42), json!(true), json!([1, 2])] {
            assert_validation_err(
                evaluate_request(v).validate(),
                "Properties must be a JSON object",
            );
        }
    }
}
