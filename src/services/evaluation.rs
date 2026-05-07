use std::sync::Arc;

use chrono::Utc;
use serde_json::Value;

use crate::errors::CustomError;
use crate::models::{
    CachedExperiment, Constraint, ConstraintOperator, Distribution, EvaluateRequest,
    ExperimentStatus, ExperimentsDB, ExposureEvent,
};
use crate::repository::db_get_experiment_by_key;
use crate::services::exposure::EventSink;

pub struct VariantAssignment {
    pub variant_key: String,
    pub config: Option<Arc<Value>>,
}

pub struct EvaluationResult {
    pub experiment_key: String,
    pub variant_key: Option<String>,
    pub config: Option<Arc<Value>>,
}

pub async fn evaluate(
    db: &ExperimentsDB,
    sink: &dyn EventSink,
    company_id: &str,
    request: EvaluateRequest,
) -> Result<EvaluationResult, CustomError> {
    let experiment = db_get_experiment_by_key(db, &request.experiment_key, company_id)
        .await?
        .filter(|exp| exp.status == ExperimentStatus::Running);

    let assignment = match experiment.as_deref() {
        Some(exp) => assign_variant(exp, &request.entity_id, &request.properties),
        None => None,
    };

    if let (Some(exp), Some(a)) = (experiment.as_deref(), &assignment) {
        sink.record_exposure(ExposureEvent::new(
            Utc::now().timestamp_millis(),
            company_id.to_string(),
            exp.experiment_id.clone(),
            Some(a.variant_key.clone()),
            request.entity_id,
        ));
    }

    let (variant_key, config) = match assignment {
        Some(a) => (Some(a.variant_key), a.config),
        None => (None, None),
    };

    Ok(EvaluationResult {
        experiment_key: request.experiment_key,
        variant_key,
        config,
    })
}

pub fn assign_variant(
    experiment: &CachedExperiment,
    entity_id: &str,
    properties: &serde_json::Value,
) -> Option<VariantAssignment> {
    // Rollout bucket is independent of the segment, so compute it once.
    let rollout_bucket = hash_to_bucket(&experiment.experiment_id, entity_id, "rollout");

    for segment in &experiment.segments {
        if !matches_constraints(properties, &segment.constraints) {
            continue;
        }
        if rollout_bucket >= segment.rollout_percent {
            continue;
        }

        if let Some(variant_key) =
            pick_variant(&segment.distributions, &experiment.experiment_id, entity_id)
        {
            let config = experiment.variant_configs.get(&variant_key).cloned();
            return Some(VariantAssignment { variant_key, config });
        }
    }

    None
}

fn matches_constraints(properties: &serde_json::Value, constraints: &[Constraint]) -> bool {
    for constraint in constraints {
        let prop_value = properties.get(&constraint.property);
        let matched = match constraint.operator {
            ConstraintOperator::Eq => match prop_value {
                Some(v) => values_eq(v, &constraint.value),
                None => false,
            },
            ConstraintOperator::Neq => match prop_value {
                Some(v) => !values_eq(v, &constraint.value),
                None => true,
            },
            ConstraintOperator::Gt => compare_values(prop_value, &constraint.value, |a, b| a > b),
            ConstraintOperator::Gte => compare_values(prop_value, &constraint.value, |a, b| a >= b),
            ConstraintOperator::Lt => compare_values(prop_value, &constraint.value, |a, b| a < b),
            ConstraintOperator::Lte => compare_values(prop_value, &constraint.value, |a, b| a <= b),
            ConstraintOperator::In => match (prop_value, constraint.value.as_array()) {
                (Some(val), Some(arr)) => arr.iter().any(|x| values_eq(x, val)),
                _ => false,
            },
            ConstraintOperator::NotIn => match (prop_value, constraint.value.as_array()) {
                (Some(val), Some(arr)) => !arr.iter().any(|x| values_eq(x, val)),
                _ => true,
            },
        };
        if !matched {
            return false;
        }
    }
    true
}

/// Equality that unifies `serde_json::Number` variants. Plain `==` on
/// `Value::Number` returns false across `PosInt` / `NegInt` / `Float`, so a
/// client property of `30` would not match a stored constraint value of
/// `30.0` (or vice versa). For other types this falls back to `Value::eq`.
fn values_eq(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value;
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            if x == y {
                return true;
            }
            match (x.as_f64(), y.as_f64()) {
                (Some(xf), Some(yf)) => xf == yf,
                _ => false,
            }
        }
        _ => a == b,
    }
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

fn pick_variant(
    distributions: &[Distribution],
    experiment_id: &str,
    entity_id: &str,
) -> Option<String> {
    if distributions.is_empty() {
        return None;
    }
    let bucket = hash_to_bucket(experiment_id, entity_id, "variant");
    let mut cumulative: u32 = 0;
    for dist in distributions {
        cumulative += dist.percent;
        if bucket < cumulative {
            return Some(dist.variant_key.clone());
        }
    }
    // Validation guarantees percentages sum to 100 and bucket is in 0..100,
    // so the loop above always returns. Reaching here means a stored
    // distribution broke that invariant — log loudly and fall back to the last
    // variant rather than silently returning "no assignment", which would
    // un-bucket an eligible user.
    log::error!(
        "pick_variant: distributions for experiment {} sum to {} (<100); falling back to last variant",
        experiment_id,
        cumulative,
    );
    distributions.last().map(|d| d.variant_key.clone())
}

/// Deterministic FNV-1a 64-bit hash, mapped to [0, 100).
///
/// FNV-1a is stable across compiler/library versions (unlike `DefaultHasher`)
/// and has good enough avalanche for non-adversarial bucketing — important so
/// the rollout salt and variant salt produce statistically independent buckets.
fn hash_to_bucket(experiment_id: &str, entity_id: &str, salt: &str) -> u32 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut h = FNV_OFFSET;
    for part in [experiment_id.as_bytes(), b":", entity_id.as_bytes(), b":", salt.as_bytes()] {
        for &b in part {
            h ^= b as u64;
            h = h.wrapping_mul(FNV_PRIME);
        }
    }
    (h % 100) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_to_bucket_in_range() {
        for i in 0..1000 {
            let entity = format!("user-{i}");
            let b = hash_to_bucket("exp-1", &entity, "rollout");
            assert!(b < 100, "bucket {} out of range for {}", b, entity);
        }
    }

    #[test]
    fn hash_to_bucket_is_deterministic() {
        let a = hash_to_bucket("exp-1", "user-42", "rollout");
        let b = hash_to_bucket("exp-1", "user-42", "rollout");
        assert_eq!(a, b);
    }

    #[test]
    fn rollout_and_variant_salts_diverge() {
        // Salts should produce different buckets for the same (experiment, entity)
        // for the vast majority of inputs. Allow some collisions but require <30%.
        let mut collisions = 0;
        let n = 1000;
        for i in 0..n {
            let entity = format!("user-{i}");
            let r = hash_to_bucket("exp-1", &entity, "rollout");
            let v = hash_to_bucket("exp-1", &entity, "variant");
            if r == v {
                collisions += 1;
            }
        }
        assert!(
            collisions < n * 30 / 100,
            "rollout/variant salts too correlated: {} collisions out of {}",
            collisions,
            n,
        );
    }

    #[test]
    fn hash_to_bucket_distribution_is_roughly_uniform() {
        let mut counts = [0u32; 10];
        let n = 10_000;
        for i in 0..n {
            let entity = format!("user-{i}");
            let b = hash_to_bucket("exp-1", &entity, "rollout");
            counts[(b / 10) as usize] += 1;
        }
        // Expected ~1000 per decile. Allow a wide tolerance — this is a smoke
        // test against pathologically biased hashes, not a statistical proof.
        for (i, &c) in counts.iter().enumerate() {
            assert!(c > 700 && c < 1300, "bucket decile {} has {} hits", i, c);
        }
    }

    #[test]
    fn pick_variant_picks_proportionally_to_distribution() {
        let dists = vec![
            Distribution { variant_key: "a".into(), percent: 30 },
            Distribution { variant_key: "b".into(), percent: 70 },
        ];
        let mut a = 0;
        let mut b = 0;
        let n = 5000;
        for i in 0..n {
            let entity = format!("user-{i}");
            match pick_variant(&dists, "exp-1", &entity).as_deref() {
                Some("a") => a += 1,
                Some("b") => b += 1,
                other => panic!("unexpected variant: {:?}", other),
            }
        }
        let a_ratio = a as f64 / n as f64;
        assert!(
            (a_ratio - 0.30).abs() < 0.05,
            "variant a ratio {} not near 0.30 (a={}, b={})",
            a_ratio,
            a,
            b,
        );
    }

    #[test]
    fn matches_constraints_eq_and_in() {
        let props = serde_json::json!({"country": "US", "tier": "pro"});
        let cs = vec![
            Constraint {
                property: "country".into(),
                operator: ConstraintOperator::Eq,
                value: serde_json::json!("US"),
            },
            Constraint {
                property: "tier".into(),
                operator: ConstraintOperator::In,
                value: serde_json::json!(["pro", "enterprise"]),
            },
        ];
        assert!(matches_constraints(&props, &cs));

        let cs_fail = vec![Constraint {
            property: "country".into(),
            operator: ConstraintOperator::Eq,
            value: serde_json::json!("DE"),
        }];
        assert!(!matches_constraints(&props, &cs_fail));
    }

    #[test]
    fn matches_constraints_numeric_ops() {
        let props = serde_json::json!({"age": 30});
        let cs = vec![Constraint {
            property: "age".into(),
            operator: ConstraintOperator::Gte,
            value: serde_json::json!(18),
        }];
        assert!(matches_constraints(&props, &cs));

        let cs = vec![Constraint {
            property: "age".into(),
            operator: ConstraintOperator::Lt,
            value: serde_json::json!(18),
        }];
        assert!(!matches_constraints(&props, &cs));
    }

    #[test]
    fn matches_constraints_eq_unifies_int_and_float() {
        // serde_json's Number doesn't compare across variants, so the raw
        // PartialEq would say `30 != 30.0`. The constraint check must.
        let cases = [
            (serde_json::json!({"age": 30}),    serde_json::json!(30.0)),
            (serde_json::json!({"age": 30.0}),  serde_json::json!(30)),
            (serde_json::json!({"age": 0}),     serde_json::json!(0.0)),
            (serde_json::json!({"age": -1}),    serde_json::json!(-1.0)),
        ];
        for (props, value) in cases {
            let cs = vec![Constraint {
                property: "age".into(),
                operator: ConstraintOperator::Eq,
                value,
            }];
            assert!(matches_constraints(&props, &cs), "eq should match for {:?}", props);
        }

        // Different numbers still don't match.
        let props = serde_json::json!({"age": 30});
        let cs = vec![Constraint {
            property: "age".into(),
            operator: ConstraintOperator::Eq,
            value: serde_json::json!(31.0),
        }];
        assert!(!matches_constraints(&props, &cs));
    }

    #[test]
    fn matches_constraints_neq_unifies_int_and_float() {
        // 30 vs 30.0 are equal → NEQ should fail.
        let props = serde_json::json!({"age": 30});
        let cs = vec![Constraint {
            property: "age".into(),
            operator: ConstraintOperator::Neq,
            value: serde_json::json!(30.0),
        }];
        assert!(!matches_constraints(&props, &cs));

        // Genuinely different numbers → NEQ passes.
        let cs = vec![Constraint {
            property: "age".into(),
            operator: ConstraintOperator::Neq,
            value: serde_json::json!(31.0),
        }];
        assert!(matches_constraints(&props, &cs));

        // Property absent → NEQ passes (preserves prior behavior).
        let props = serde_json::json!({});
        let cs = vec![Constraint {
            property: "age".into(),
            operator: ConstraintOperator::Neq,
            value: serde_json::json!(30),
        }];
        assert!(matches_constraints(&props, &cs));
    }

    #[test]
    fn matches_constraints_in_unifies_int_and_float() {
        // Integer property in float-typed array.
        let props = serde_json::json!({"tier": 2});
        let cs = vec![Constraint {
            property: "tier".into(),
            operator: ConstraintOperator::In,
            value: serde_json::json!([1.0, 2.0, 3.0]),
        }];
        assert!(matches_constraints(&props, &cs));

        // Float property in integer-typed array.
        let props = serde_json::json!({"tier": 2.0});
        let cs = vec![Constraint {
            property: "tier".into(),
            operator: ConstraintOperator::In,
            value: serde_json::json!([1, 2, 3]),
        }];
        assert!(matches_constraints(&props, &cs));

        // NOT_IN mirrors IN.
        let props = serde_json::json!({"tier": 4});
        let cs = vec![Constraint {
            property: "tier".into(),
            operator: ConstraintOperator::NotIn,
            value: serde_json::json!([1.0, 2.0, 3.0]),
        }];
        assert!(matches_constraints(&props, &cs));
    }

    #[test]
    fn values_eq_strings_and_bools_unaffected() {
        // Sanity: non-numeric paths still go through Value::eq.
        assert!(values_eq(&serde_json::json!("a"), &serde_json::json!("a")));
        assert!(!values_eq(&serde_json::json!("a"), &serde_json::json!("b")));
        assert!(values_eq(&serde_json::json!(true), &serde_json::json!(true)));
        assert!(!values_eq(&serde_json::json!(true), &serde_json::json!(1)));
    }
}
