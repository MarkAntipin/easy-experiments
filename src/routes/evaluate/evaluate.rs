use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{
    ExperimentsDB, ExperimentStatus, EvaluateRequest, EvaluateResponse,
    Constraint, ConstraintOperator, Distribution, Segment, Variant,
};
use crate::repository::db_get_experiment_by_key;
use crate::validation::ValidatedJson;

pub async fn evaluate(
    db: web::Data<ExperimentsDB>,
    payload: ValidatedJson<EvaluateRequest>,
) -> Result<HttpResponse, CustomError> {
    let request = payload.into_inner();

    let experiment = match db_get_experiment_by_key(&db, &request.experiment_key)
        .await
        .map_err(CustomError::from)?
    {
        Some(r) => r,
        None => {
            return Ok(HttpResponse::Ok().json(EvaluateResponse {
                experiment_key: request.experiment_key,
                variant_key: None,
                attachment: None,
            }));
        }
    };

    if experiment.status != ExperimentStatus::Running {
        return Ok(HttpResponse::Ok().json(EvaluateResponse {
            experiment_key: request.experiment_key,
            variant_key: None,
            attachment: None,
        }));
    }

    let variants: Vec<Variant> =
        serde_json::from_str(&experiment.variants).unwrap_or_default();
    let mut segments: Vec<Segment> =
        serde_json::from_str(&experiment.segments).unwrap_or_default();
    segments.sort_by_key(|s| s.rank);

    for segment in &segments {
        if !matches_constraints(&request.properties, &segment.constraints) {
            continue;
        }

        let rollout_bucket = hash_to_bucket(&request.experiment_key, &request.entity_id, "rollout");
        if rollout_bucket >= segment.rollout_percent {
            continue;
        }

        if let Some(variant_key) = pick_variant(&segment.distributions, &request.experiment_key, &request.entity_id) {
            let attachment = variants
                .iter()
                .find(|v| v.key == variant_key)
                .map(|v| v.attachment.clone());

            return Ok(HttpResponse::Ok().json(EvaluateResponse {
                experiment_key: request.experiment_key,
                variant_key: Some(variant_key),
                attachment,
            }));
        }
    }

    Ok(HttpResponse::Ok().json(EvaluateResponse {
        experiment_key: request.experiment_key,
        variant_key: None,
        attachment: None,
    }))
}

fn matches_constraints(properties: &serde_json::Value, constraints: &[Constraint]) -> bool {
    for constraint in constraints {
        let prop_value = properties.get(&constraint.property);
        let matched = match constraint.operator {
            ConstraintOperator::Eq => prop_value == Some(&constraint.value),
            ConstraintOperator::Neq => prop_value != Some(&constraint.value),
            ConstraintOperator::Gt => compare_values(prop_value, &constraint.value, |a, b| a > b),
            ConstraintOperator::Gte => compare_values(prop_value, &constraint.value, |a, b| a >= b),
            ConstraintOperator::Lt => compare_values(prop_value, &constraint.value, |a, b| a < b),
            ConstraintOperator::Lte => compare_values(prop_value, &constraint.value, |a, b| a <= b),
            ConstraintOperator::In => {
                if let (Some(val), Some(arr)) = (prop_value, constraint.value.as_array()) {
                    arr.contains(val)
                } else {
                    false
                }
            }
            ConstraintOperator::NotIn => {
                if let (Some(val), Some(arr)) = (prop_value, constraint.value.as_array()) {
                    !arr.contains(val)
                } else {
                    true
                }
            }
        };
        if !matched {
            return false;
        }
    }
    true
}

fn compare_values(
    prop: Option<&serde_json::Value>,
    constraint_val: &serde_json::Value,
    cmp: fn(f64, f64) -> bool,
) -> bool {
    match (prop.and_then(|v| v.as_f64()), constraint_val.as_f64()) {
        (Some(a), Some(b)) => cmp(a, b),
        _ => false,
    }
}

fn pick_variant(distributions: &[Distribution], experiment_key: &str, entity_id: &str) -> Option<String> {
    let bucket = hash_to_bucket(experiment_key, entity_id, "variant");
    let mut cumulative: u32 = 0;
    for dist in distributions {
        cumulative += dist.percent;
        if bucket < cumulative {
            return Some(dist.variant_key.clone());
        }
    }
    distributions
        .last()
        .map(|d| d.variant_key.clone())
}

/// Deterministic hash for bucketing. Returns 0..99.
fn hash_to_bucket(experiment_key: &str, entity_id: &str, salt: &str) -> u32 {
    let input = format!("{}:{}:{}", experiment_key, entity_id, salt);
    let mut hash: u32 = 0;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }
    hash % 100
}
