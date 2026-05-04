use std::collections::HashSet;

use serde::{Deserialize, Deserializer, Serialize};

use crate::errors::CustomError;
use crate::models::domain::{ConstraintOperator, Segment, Variant};
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
const MAX_VARIANT_CONFIG_BYTES: usize = 4096;
const MAX_CONSTRAINT_PROPERTY_LENGTH: usize = 256;
const MAX_CONSTRAINT_VALUE_BYTES: usize = 1024;
const MAX_VARIANTS: usize = 64;
const MAX_SEGMENTS: usize = 64;
const MAX_DISTRIBUTIONS_PER_SEGMENT: usize = 64;
const MAX_CONSTRAINTS_PER_SEGMENT: usize = 64;
const MAX_ENTITY_ID_LENGTH: usize = 256;
pub const MAX_IDEMPOTENCY_KEY_LENGTH: usize = 256;

/// Reject control characters and any whitespace in user-supplied identifiers
/// that get echoed in error messages, stored in DB, or written to logs. Keeps
/// `\n`, `\r`, `\x1b` (ANSI escapes), NUL, etc. out of `journalctl` output.
fn validate_safe_identifier(field: &str, value: &str) -> Result<(), CustomError> {
    if value.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(CustomError::ValidationError(format!(
            "{} must not contain whitespace or control characters",
            field
        )));
    }
    Ok(())
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

pub(crate) fn validate_experiment_state(
    description: Option<&str>,
    primary_metric: &str,
    variants: &[Variant],
    segments: &[Segment],
) -> Result<(), CustomError> {
    validate_description(description)?;
    validate_primary_metric(primary_metric)?;
    validate_variants(variants)?;
    validate_segments(segments, variants)?;
    Ok(())
}

fn validate_description(description: Option<&str>) -> Result<(), CustomError> {
    if let Some(desc) = description {
        if desc.is_empty() {
            return Err(CustomError::ValidationError(
                "Description should not be empty; omit the field or send null to clear it".into(),
            ));
        }
        if desc != desc.trim() {
            return Err(CustomError::ValidationError(
                "Description must not have leading or trailing whitespace".into(),
            ));
        }
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
    if key.len() > MAX_KEY_LENGTH {
        return Err(CustomError::ValidationError(format!(
            "Key length should be less than {} bytes", MAX_KEY_LENGTH
        )));
    }
    validate_safe_identifier("Key", key)
}

pub fn validate_idempotency_key(key: &str) -> Result<(), CustomError> {
    if key.is_empty() {
        return Err(CustomError::ValidationError(
            "Idempotency-Key header must not be empty".into(),
        ));
    }
    if key.len() > MAX_IDEMPOTENCY_KEY_LENGTH {
        return Err(CustomError::ValidationError(format!(
            "Idempotency-Key length should be less than {} bytes",
            MAX_IDEMPOTENCY_KEY_LENGTH
        )));
    }
    validate_safe_identifier("Idempotency-Key", key)
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
    validate_safe_identifier("Primary metric", primary_metric)
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
        validate_safe_identifier("Variant key", &variant.key)?;
        if !seen.insert(variant.key.as_str()) {
            return Err(CustomError::ValidationError(format!(
                "Duplicate variant key '{}'", variant.key
            )));
        }
        let config_bytes = serde_json::to_vec(&variant.config)
            .map_err(|e| {
                CustomError::InternalError(format!("Failed to serialize config: {}", e))
            })?
            .len();
        if config_bytes > MAX_VARIANT_CONFIG_BYTES {
            return Err(CustomError::ValidationError(format!(
                "Variant config size should be less than {} bytes",
                MAX_VARIANT_CONFIG_BYTES
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

fn validate_segments(segments: &[Segment], variants: &[Variant]) -> Result<(), CustomError> {
    if segments.is_empty() {
        return Err(CustomError::ValidationError("Must have at least one segment".into()));
    }
    if segments.len() > MAX_SEGMENTS {
        return Err(CustomError::ValidationError(format!(
            "Must have at most {} segments", MAX_SEGMENTS
        )));
    }

    let variant_keys: HashSet<&str> = variants.iter().map(|v| v.key.as_str()).collect();

    let mut seen_priorities = HashSet::with_capacity(segments.len());
    for segment in segments {
        if segment.priority < 0 {
            return Err(CustomError::ValidationError(
                "Segment priority must be non-negative".into(),
            ));
        }
        if !seen_priorities.insert(segment.priority) {
            return Err(CustomError::ValidationError(format!(
                "Duplicate segment priority '{}'", segment.priority
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
            validate_safe_identifier("Constraint property", &constraint.property)?;
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
        let mut seen_dist_keys = HashSet::with_capacity(segment.distributions.len());
        for dist in &segment.distributions {
            if dist.percent > 100 {
                return Err(CustomError::ValidationError(
                    "Distribution percent must be between 0 and 100".into(),
                ));
            }
            if !variant_keys.contains(dist.variant_key.as_str()) {
                return Err(CustomError::ValidationError(format!(
                    "Distribution references unknown variant key '{}'", dist.variant_key
                )));
            }
            if !seen_dist_keys.insert(dist.variant_key.as_str()) {
                return Err(CustomError::ValidationError(format!(
                    "Duplicate distribution for variant key '{}' in segment", dist.variant_key
                )));
            }
            total = total.saturating_add(dist.percent);
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
            None => Err(CustomError::ValidationError(
                "IN/NOT_IN constraint value must be an array".into(),
            )),
            Some(arr) if arr.is_empty() => Err(CustomError::ValidationError(
                "IN/NOT_IN constraint value must be a non-empty array".into(),
            )),
            Some(arr) => {
                if arr
                    .iter()
                    .all(|v| matches!(v, Value::String(_) | Value::Number(_) | Value::Bool(_)))
                {
                    Ok(())
                } else {
                    Err(CustomError::ValidationError(
                        "IN/NOT_IN constraint value array elements must be strings, numbers, or booleans".into(),
                    ))
                }
            }
        },
    }
}

#[derive(Serialize, Deserialize)]
pub struct GoogleLoginRequest {
    pub token: String,
}

const MAX_API_KEY_NAME_LENGTH: usize = 128;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyRequest {
    pub name: String,
}

impl Validate for CreateApiKeyRequest {
    fn validate(&self) -> Result<(), CustomError> {
        if self.name.is_empty() {
            return Err(CustomError::ValidationError(
                "API key name should not be empty".into(),
            ));
        }
        if self.name != self.name.trim() {
            return Err(CustomError::ValidationError(
                "API key name must not have leading or trailing whitespace".into(),
            ));
        }
        if self.name.len() > MAX_API_KEY_NAME_LENGTH {
            return Err(CustomError::ValidationError(format!(
                "API key name length should be less than {} bytes",
                MAX_API_KEY_NAME_LENGTH
            )));
        }
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::domain::{Constraint, Distribution};
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

    fn variant(key: &str, is_control: bool) -> Variant {
        Variant {
            key: key.into(),
            is_control,
            config: json!({}),
        }
    }

    fn distribution(key: &str, percent: u32) -> Distribution {
        Distribution { variant_key: key.into(), percent }
    }

    fn simple_segment() -> Segment {
        Segment {
            priority: 0,
            rollout_percent: 100,
            constraints: vec![],
            distributions: vec![
                distribution("control", 50),
                distribution("treatment", 50),
            ],
        }
    }

    fn simple_variants() -> Vec<Variant> {
        vec![variant("control", true), variant("treatment", false)]
    }

    // ---- validate_key ----

    #[test]
    fn validate_key_ok() {
        assert!(validate_key("abc").is_ok());
    }

    #[test]
    fn validate_key_rejects_empty_whitespace_and_overlong() {
        assert_validation_err(validate_key(""), "empty");
        assert_validation_err(validate_key(" abc"), "whitespace");
        assert_validation_err(validate_key("abc "), "whitespace");
        let too_long = "a".repeat(MAX_KEY_LENGTH + 1);
        assert_validation_err(validate_key(&too_long), "Key length");
        assert!(validate_key(&"a".repeat(MAX_KEY_LENGTH)).is_ok());
    }

    // ---- validate_primary_metric ----

    #[test]
    fn validate_primary_metric_ok_and_errors() {
        assert!(validate_primary_metric("clicks").is_ok());
        assert_validation_err(validate_primary_metric(""), "empty");
        assert_validation_err(validate_primary_metric(" clicks"), "whitespace");
        assert_validation_err(
            validate_primary_metric(&"m".repeat(MAX_PRIMARY_METRIC_LENGTH + 1)),
            "Primary metric length",
        );
    }

    // ---- validate_description ----

    #[test]
    fn validate_description_none_is_ok() {
        assert!(validate_description(None).is_ok());
    }

    #[test]
    fn validate_description_errors() {
        assert_validation_err(validate_description(Some("")), "empty");
        assert_validation_err(validate_description(Some(" x")), "whitespace");
        assert_validation_err(validate_description(Some("x ")), "whitespace");
        let too_long = "d".repeat(MAX_DESCRIPTION_LENGTH + 1);
        assert_validation_err(validate_description(Some(&too_long)), "Description length");
        assert!(validate_description(Some("ok")).is_ok());
    }

    // ---- validate_variants ----

    #[test]
    fn validate_variants_happy_path() {
        assert!(validate_variants(&simple_variants()).is_ok());
    }

    #[test]
    fn validate_variants_requires_non_empty() {
        assert_validation_err(validate_variants(&[]), "empty");
    }

    #[test]
    fn validate_variants_requires_exactly_one_control() {
        // zero controls
        let v = vec![variant("a", false), variant("b", false)];
        assert_validation_err(validate_variants(&v), "Exactly one");
        // two controls
        let v = vec![variant("a", true), variant("b", true)];
        assert_validation_err(validate_variants(&v), "Exactly one");
    }

    #[test]
    fn validate_variants_rejects_duplicate_keys() {
        let v = vec![variant("dup", true), variant("dup", false)];
        assert_validation_err(validate_variants(&v), "Duplicate variant key");
    }

    #[test]
    fn validate_variants_rejects_bad_key_shape() {
        let v = vec![variant("", true), variant("b", false)];
        assert_validation_err(validate_variants(&v), "empty");
        let v = vec![variant(" a", true), variant("b", false)];
        assert_validation_err(validate_variants(&v), "whitespace");
        let long = "k".repeat(MAX_VARIANT_KEY_LENGTH + 1);
        let v = vec![variant(&long, true), variant("b", false)];
        assert_validation_err(validate_variants(&v), "Variant key length");
    }

    #[test]
    fn validate_variants_rejects_too_many() {
        let mut v: Vec<Variant> = (0..=MAX_VARIANTS)
            .map(|i| variant(&format!("v{i}"), false))
            .collect();
        v[0].is_control = true;
        assert_validation_err(validate_variants(&v), "at most");
    }

    #[test]
    fn validate_variants_rejects_oversized_config() {
        let mut v = simple_variants();
        v[0].config = json!({ "blob": "x".repeat(MAX_VARIANT_CONFIG_BYTES) });
        assert_validation_err(validate_variants(&v), "config size");
    }

    // ---- validate_segments ----

    #[test]
    fn validate_segments_happy_path() {
        let variants = simple_variants();
        assert!(validate_segments(&[simple_segment()], &variants).is_ok());
    }

    #[test]
    fn validate_segments_requires_non_empty() {
        let variants = simple_variants();
        assert_validation_err(validate_segments(&[], &variants), "at least one segment");
    }

    #[test]
    fn validate_segments_distributions_must_sum_to_100() {
        let variants = simple_variants();
        let seg = Segment {
            priority: 0,
            rollout_percent: 100,
            constraints: vec![],
            distributions: vec![distribution("control", 50), distribution("treatment", 40)],
        };
        assert_validation_err(validate_segments(&[seg], &variants), "sum to 100");
    }

    #[test]
    fn validate_segments_rejects_unknown_variant_reference() {
        let variants = simple_variants();
        let seg = Segment {
            priority: 0,
            rollout_percent: 100,
            constraints: vec![],
            distributions: vec![distribution("ghost", 100)],
        };
        assert_validation_err(validate_segments(&[seg], &variants), "unknown variant");
    }

    #[test]
    fn validate_segments_rejects_duplicate_distribution_keys() {
        let variants = simple_variants();
        let seg = Segment {
            priority: 0,
            rollout_percent: 100,
            constraints: vec![],
            distributions: vec![distribution("control", 50), distribution("control", 50)],
        };
        assert_validation_err(validate_segments(&[seg], &variants), "Duplicate distribution");
    }

    #[test]
    fn validate_segments_rejects_duplicate_priorities() {
        let variants = simple_variants();
        let segs = vec![simple_segment(), simple_segment()]; // both priority 0
        assert_validation_err(validate_segments(&segs, &variants), "Duplicate segment priority");
    }

    #[test]
    fn validate_segments_rejects_negative_priority_and_bad_rollout() {
        let variants = simple_variants();
        let seg = Segment { priority: -1, ..simple_segment() };
        assert_validation_err(validate_segments(&[seg], &variants), "non-negative");

        let seg = Segment { rollout_percent: 101, ..simple_segment() };
        assert_validation_err(validate_segments(&[seg], &variants), "between 0 and 100");
    }

    #[test]
    fn validate_segments_rejects_bad_distribution_percent() {
        let variants = simple_variants();
        let seg = Segment {
            priority: 0,
            rollout_percent: 100,
            constraints: vec![],
            distributions: vec![distribution("control", 101)],
        };
        assert_validation_err(validate_segments(&[seg], &variants), "between 0 and 100");
    }

    #[test]
    fn validate_segments_validates_constraint_property_shape() {
        let variants = simple_variants();
        let seg = Segment {
            priority: 0,
            rollout_percent: 100,
            constraints: vec![Constraint {
                property: "".into(),
                operator: ConstraintOperator::Eq,
                value: json!("x"),
            }],
            distributions: vec![distribution("control", 50), distribution("treatment", 50)],
        };
        assert_validation_err(validate_segments(&[seg], &variants), "property must not be empty");

        let seg = Segment {
            priority: 0,
            rollout_percent: 100,
            constraints: vec![Constraint {
                property: "p".repeat(MAX_CONSTRAINT_PROPERTY_LENGTH + 1),
                operator: ConstraintOperator::Eq,
                value: json!("x"),
            }],
            distributions: vec![distribution("control", 50), distribution("treatment", 50)],
        };
        assert_validation_err(validate_segments(&[seg], &variants), "property length");
    }

    // ---- validate_constraint_value_shape ----

    #[test]
    fn constraint_eq_neq_accepts_primitives_rejects_compound() {
        for op in [ConstraintOperator::Eq, ConstraintOperator::Neq] {
            assert!(validate_constraint_value_shape(op, &json!("s")).is_ok());
            assert!(validate_constraint_value_shape(op, &json!(1)).is_ok());
            assert!(validate_constraint_value_shape(op, &json!(true)).is_ok());
            assert_validation_err(
                validate_constraint_value_shape(op, &json!([1, 2])),
                "string, number, or boolean",
            );
            assert_validation_err(
                validate_constraint_value_shape(op, &json!({"a": 1})),
                "string, number, or boolean",
            );
            assert_validation_err(
                validate_constraint_value_shape(op, &json!(null)),
                "string, number, or boolean",
            );
        }
    }

    #[test]
    fn constraint_numeric_ops_require_number() {
        for op in [
            ConstraintOperator::Gt,
            ConstraintOperator::Gte,
            ConstraintOperator::Lt,
            ConstraintOperator::Lte,
        ] {
            assert!(validate_constraint_value_shape(op, &json!(1)).is_ok());
            assert!(validate_constraint_value_shape(op, &json!(1.5)).is_ok());
            assert_validation_err(
                validate_constraint_value_shape(op, &json!("1")),
                "must be a number",
            );
            assert_validation_err(
                validate_constraint_value_shape(op, &json!(true)),
                "must be a number",
            );
        }
    }

    #[test]
    fn constraint_in_notin_require_non_empty_primitive_array() {
        for op in [ConstraintOperator::In, ConstraintOperator::NotIn] {
            assert!(validate_constraint_value_shape(op, &json!([1, "a", true])).is_ok());
            assert_validation_err(
                validate_constraint_value_shape(op, &json!("a")),
                "must be an array",
            );
            assert_validation_err(
                validate_constraint_value_shape(op, &json!([])),
                "non-empty array",
            );
            assert_validation_err(
                validate_constraint_value_shape(op, &json!([1, {"x": 1}])),
                "array elements must be",
            );
            assert_validation_err(
                validate_constraint_value_shape(op, &json!([1, null])),
                "array elements must be",
            );
        }
    }

    // ---- EvaluateRequest::validate ----

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
