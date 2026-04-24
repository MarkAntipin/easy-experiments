use std::collections::HashSet;

use serde::{Deserialize, Deserializer, Serialize};

use crate::errors::CustomError;
use crate::models::db::{ConstraintOperator, Segment, Variant};
use crate::validation::Validate;

fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

const MAX_KEY_LENGTH: usize = 256;
const MAX_DESCRIPTION_LENGTH: usize = 4096;
const MAX_PRIMARY_METRIC_LENGTH: usize = 256;
const MAX_VARIANT_KEY_LENGTH: usize = 256;
const MAX_CONSTRAINT_PROPERTY_LENGTH: usize = 256;
const MAX_CONSTRAINT_VALUE_BYTES: usize = 4096;
const MAX_VARIANTS: usize = 64;
const MAX_SEGMENTS: usize = 64;
const MAX_DISTRIBUTIONS_PER_SEGMENT: usize = 64;
const MAX_CONSTRAINTS_PER_SEGMENT: usize = 64;

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
        Ok(())
    }
}

pub(crate) fn validate_experiment_state(
    description: Option<&str>,
    primary_metric: &str,
    variants: &[Variant],
    segments: &[Segment],
) -> Result<(), CustomError> {
    validate_description(description)?;
    validate_primary_metric(primary_metric)?;
    validate_variants(variants)?;
    validate_segments(segments, Some(variants))?;
    Ok(())
}

fn validate_description(description: Option<&str>) -> Result<(), CustomError> {
    if let Some(desc) = description {
        if desc.len() > MAX_DESCRIPTION_LENGTH {
            return Err(CustomError::ValidationError(format!(
                "Description length should be less than {} bytes", MAX_DESCRIPTION_LENGTH
            )));
        }
    }
    Ok(())
}

fn validate_key(key: &str) -> Result<(), CustomError> {
    if key.is_empty() {
        return Err(CustomError::ValidationError("Key should not be empty".into()));
    }
    if key != key.trim() {
        return Err(CustomError::ValidationError(
            "Key must not have leading or trailing whitespace".into(),
        ));
    }
    if key.len() > MAX_KEY_LENGTH {
        return Err(CustomError::ValidationError(format!(
            "Key length should be less than {} bytes", MAX_KEY_LENGTH
        )));
    }
    Ok(())
}

fn validate_primary_metric(primary_metric: &str) -> Result<(), CustomError> {
    if primary_metric.is_empty() {
        return Err(CustomError::ValidationError("Primary metric should not be empty".into()));
    }
    if primary_metric.len() > MAX_PRIMARY_METRIC_LENGTH {
        return Err(CustomError::ValidationError(format!(
            "Primary metric length should be less than {} bytes", MAX_PRIMARY_METRIC_LENGTH
        )));
    }
    Ok(())
}

fn validate_variants(variants: &[Variant]) -> Result<(), CustomError> {
    if variants.is_empty() {
        return Err(CustomError::ValidationError("Variants should not be empty".into()));
    }
    if variants.len() > MAX_VARIANTS {
        return Err(CustomError::ValidationError(format!(
            "Must have at most {} variants", MAX_VARIANTS
        )));
    }

    let mut seen = HashSet::with_capacity(variants.len());
    let mut control_count = 0;
    for variant in variants {
        if variant.key.is_empty() {
            return Err(CustomError::ValidationError("Variant key should not be empty".into()));
        }
        if variant.key.len() > MAX_VARIANT_KEY_LENGTH {
            return Err(CustomError::ValidationError(format!(
                "Variant key length should be less than {} bytes", MAX_VARIANT_KEY_LENGTH
            )));
        }
        if !seen.insert(variant.key.as_str()) {
            return Err(CustomError::ValidationError(format!(
                "Duplicate variant key '{}'", variant.key
            )));
        }
        if variant.is_control {
            control_count += 1;
        }
    }
    if control_count != 1 {
        return Err(CustomError::ValidationError(
            "Exactly one variant must be marked as control".into(),
        ));
    }
    Ok(())
}

fn validate_segments(segments: &[Segment], variants: Option<&[Variant]>) -> Result<(), CustomError> {
    if segments.is_empty() {
        return Err(CustomError::ValidationError("Must have at least one segment".into()));
    }
    if segments.len() > MAX_SEGMENTS {
        return Err(CustomError::ValidationError(format!(
            "Must have at most {} segments", MAX_SEGMENTS
        )));
    }

    let variant_keys: Option<HashSet<&str>> = variants
        .map(|vs| vs.iter().map(|v| v.key.as_str()).collect());

    let mut seen_ranks = HashSet::with_capacity(segments.len());
    for segment in segments {
        if segment.rank < 0 {
            return Err(CustomError::ValidationError(
                "Segment rank must be non-negative".into(),
            ));
        }
        if !seen_ranks.insert(segment.rank) {
            return Err(CustomError::ValidationError(format!(
                "Duplicate segment rank '{}'", segment.rank
            )));
        }
        if segment.rollout_percent > 100 {
            return Err(CustomError::ValidationError(
                "Segment rollout_percent must be between 0 and 100".into(),
            ));
        }
        if segment.constraints.len() > MAX_CONSTRAINTS_PER_SEGMENT {
            return Err(CustomError::ValidationError(format!(
                "Segment must have at most {} constraints", MAX_CONSTRAINTS_PER_SEGMENT
            )));
        }
        for constraint in &segment.constraints {
            if constraint.property.is_empty() {
                return Err(CustomError::ValidationError(
                    "Constraint property must not be empty".into(),
                ));
            }
            if constraint.property.len() > MAX_CONSTRAINT_PROPERTY_LENGTH {
                return Err(CustomError::ValidationError(format!(
                    "Constraint property length should be less than {} bytes",
                    MAX_CONSTRAINT_PROPERTY_LENGTH
                )));
            }
            if constraint.value.to_string().len() > MAX_CONSTRAINT_VALUE_BYTES {
                return Err(CustomError::ValidationError(format!(
                    "Constraint value length should be less than {} bytes",
                    MAX_CONSTRAINT_VALUE_BYTES
                )));
            }
            validate_constraint_value_shape(constraint.operator, &constraint.value)?;
        }
        if segment.distributions.is_empty() {
            return Err(CustomError::ValidationError(
                "Segment must have at least one distribution".into(),
            ));
        }
        if segment.distributions.len() > MAX_DISTRIBUTIONS_PER_SEGMENT {
            return Err(CustomError::ValidationError(format!(
                "Segment must have at most {} distributions", MAX_DISTRIBUTIONS_PER_SEGMENT
            )));
        }
        let mut total: u32 = 0;
        for dist in &segment.distributions {
            if dist.percent > 100 {
                return Err(CustomError::ValidationError(
                    "Distribution percent must be between 0 and 100".into(),
                ));
            }
            total = total.saturating_add(dist.percent);
            if let Some(ref keys) = variant_keys {
                if !keys.contains(dist.variant_key.as_str()) {
                    return Err(CustomError::ValidationError(format!(
                        "Distribution references unknown variant key '{}'", dist.variant_key
                    )));
                }
            }
        }
        if total != 100 {
            return Err(CustomError::ValidationError(
                "Distribution percentages must sum to 100".into(),
            ));
        }
    }
    Ok(())
}

fn validate_constraint_value_shape(
    operator: ConstraintOperator,
    value: &serde_json::Value,
) -> Result<(), CustomError> {
    use serde_json::Value;
    match operator {
        ConstraintOperator::Eq | ConstraintOperator::Neq => match value {
            Value::String(_) | Value::Number(_) | Value::Bool(_) => Ok(()),
            _ => Err(CustomError::ValidationError(
                "EQ/NEQ constraint value must be a string, number, or boolean".into(),
            )),
        },
        ConstraintOperator::Gt
        | ConstraintOperator::Gte
        | ConstraintOperator::Lt
        | ConstraintOperator::Lte => {
            if value.is_number() {
                Ok(())
            } else {
                Err(CustomError::ValidationError(
                    "GT/GTE/LT/LTE constraint value must be a number".into(),
                ))
            }
        }
        ConstraintOperator::In | ConstraintOperator::NotIn => match value.as_array() {
            Some(arr) if !arr.is_empty() => Ok(()),
            Some(_) => Err(CustomError::ValidationError(
                "IN/NOT_IN constraint value must be a non-empty array".into(),
            )),
            None => Err(CustomError::ValidationError(
                "IN/NOT_IN constraint value must be an array".into(),
            )),
        },
    }
}

#[derive(Serialize, Deserialize)]
pub struct GoogleLoginRequest {
    pub token: String,
}

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
        if self.entity_id.is_empty() {
            return Err(CustomError::ValidationError("Entity ID should not be empty".into()));
        }
        Ok(())
    }
}
