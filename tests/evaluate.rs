//! Integration tests for `POST /api/v1/experiments/evaluate`.
//!
//! Exercises the API-key middleware, body validation, the evaluation
//! service, and the response envelope end-to-end against a real (in-memory)
//! SQLite database.
//!
//! Run with:   cargo test --test evaluate

mod common;

use reqwest::StatusCode;
use serde_json::{json, Value};

use crate::common::{valid_experiment_body, TestApp};

/// Minimal running experiment: 100% rollout, 50/50 split, no constraints.
/// Useful for tests that want a deterministic non-null assignment.
async fn running_minimal(app: &TestApp, key: &str) -> String {
    app.running_experiment(&valid_experiment_body(key)).await
}

// -- Auth --------------------------------------------------------------------

#[tokio::test]
async fn evaluate_missing_api_key_unauthorized() {
    // arrange
    let app = TestApp::spawn().await;

    // act: no X-Api-Key header at all
    let response = app
        .raw_client()
        .post(format!("{}/api/v1/experiments/evaluate", app.addr()))
        .json(&json!({"experimentKey": "x", "entityId": "u1"}))
        .send()
        .await
        .unwrap();

    // assert
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn evaluate_unknown_api_key_unauthorized() {
    // arrange: plausible-looking key (correct prefix + body) but never seeded.
    let app = TestApp::spawn().await;

    // act
    let response = app
        .evaluate(
            "eek-thiskeydoesnotexistinthedatabase",
            &json!({"experimentKey": "x", "entityId": "u1"}),
        )
        .await;

    // assert
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn evaluate_malformed_api_key_unauthorized() {
    // arrange: bypasses the SQLite lookup via `is_plausible_api_key` — guards
    // against accidental loosening of the prefix check.
    let app = TestApp::spawn().await;

    // act
    let response = app
        .evaluate(
            "not-an-eek-key",
            &json!({"experimentKey": "x", "entityId": "u1"}),
        )
        .await;

    // assert
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// -- Validation --------------------------------------------------------------

#[tokio::test]
async fn evaluate_empty_entity_id_validation_error() {
    // arrange
    let app = TestApp::spawn().await;
    let api_key = app.seed_api_key().await;

    // act
    let response = app
        .evaluate(&api_key, &json!({"experimentKey": "x", "entityId": ""}))
        .await;

    // assert
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn evaluate_non_object_properties_validation_error() {
    // arrange
    let app = TestApp::spawn().await;
    let api_key = app.seed_api_key().await;

    // act
    let response = app
        .evaluate(
            &api_key,
            &json!({"experimentKey": "x", "entityId": "u1", "properties": [1, 2]}),
        )
        .await;

    // assert
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// -- Assignment --------------------------------------------------------------

#[tokio::test]
async fn evaluate_unknown_experiment_null_variant() {
    // arrange: "dumb client" contract — missing experiment is not an error,
    // just no assignment. Captures the current behavior so a future change
    // is loud.
    let app = TestApp::spawn().await;
    let api_key = app.seed_api_key().await;

    // act
    let response = app
        .evaluate(
            &api_key,
            &json!({"experimentKey": "does-not-exist", "entityId": "u1"}),
        )
        .await;

    // assert
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["experimentKey"], "does-not-exist");
    assert!(body["variantKey"].is_null());
    assert!(body["config"].is_null());
}

#[tokio::test]
async fn evaluate_draft_experiment_null_variant() {
    // arrange: only Running experiments produce assignments.
    let app = TestApp::spawn().await;
    let api_key = app.seed_api_key().await;
    let body = valid_experiment_body("not_started");
    let create = app.post_experiment(&body).await;
    assert_eq!(create.status(), StatusCode::CREATED);

    // act
    let response = app
        .evaluate(
            &api_key,
            &json!({"experimentKey": "not_started", "entityId": "u1"}),
        )
        .await;

    // assert
    assert_eq!(response.status(), StatusCode::OK);
    let payload: Value = response.json().await.unwrap();
    assert!(payload["variantKey"].is_null());
}

#[tokio::test]
async fn evaluate_running_experiment_assigns_variant() {
    // arrange
    let app = TestApp::spawn().await;
    let api_key = app.seed_api_key().await;
    running_minimal(&app, "running_exp").await;

    // act
    let response = app
        .evaluate(
            &api_key,
            &json!({"experimentKey": "running_exp", "entityId": "user-1"}),
        )
        .await;

    // assert
    assert_eq!(response.status(), StatusCode::OK);
    let payload: Value = response.json().await.unwrap();
    assert_eq!(payload["experimentKey"], "running_exp");
    let variant = payload["variantKey"].as_str().expect("variantKey set");
    assert!(matches!(variant, "control" | "treatment"));
    // valid_experiment_body uses `config: {}`, so we expect an empty object.
    assert_eq!(payload["config"], json!({}));
}

#[tokio::test]
async fn evaluate_same_entity_deterministic() {
    // arrange
    let app = TestApp::spawn().await;
    let api_key = app.seed_api_key().await;
    running_minimal(&app, "deterministic").await;
    let req = json!({"experimentKey": "deterministic", "entityId": "stable-user"});

    // act
    let first: Value = app.evaluate(&api_key, &req).await.json().await.unwrap();
    let second: Value = app.evaluate(&api_key, &req).await.json().await.unwrap();

    // assert
    assert_eq!(first["variantKey"], second["variantKey"]);
}

#[tokio::test]
async fn evaluate_zero_rollout_null_variant() {
    // arrange: rolloutPercent: 0 means no entity ever lands in this segment.
    let app = TestApp::spawn().await;
    let api_key = app.seed_api_key().await;
    let mut body = valid_experiment_body("no_rollout");
    body["segments"][0]["rolloutPercent"] = json!(0);
    app.running_experiment(&body).await;

    // act + assert: 50 entities, none should be bucketed.
    for i in 0..50 {
        let payload: Value = app
            .evaluate(
                &api_key,
                &json!({"experimentKey": "no_rollout", "entityId": format!("u-{i}")}),
            )
            .await
            .json()
            .await
            .unwrap();
        assert!(
            payload["variantKey"].is_null(),
            "entity {i} should not have been bucketed: {payload:?}"
        );
    }
}

#[tokio::test]
async fn evaluate_other_tenant_null_variant() {
    // arrange: tenant A's experiment must be invisible to tenant B's API key.
    // Same key collision across companies must NOT match — the lookup is
    // scoped by company_id.
    let app = TestApp::spawn().await;
    let api_key_a = app.seed_api_key().await;
    running_minimal(&app, "tenant_isolated").await;
    let (other_user, _) = app.seed_other_user().await;
    let api_key_b = app.seed_api_key_for(&other_user.company_id).await;
    let req = json!({"experimentKey": "tenant_isolated", "entityId": "u1"});

    // act
    let resp_a: Value = app.evaluate(&api_key_a, &req).await.json().await.unwrap();
    let resp_b: Value = app.evaluate(&api_key_b, &req).await.json().await.unwrap();

    // assert: same key works for tenant A but tenant B sees null.
    assert!(resp_a["variantKey"].as_str().is_some());
    assert!(resp_b["variantKey"].is_null());
}

// -- Constraints -------------------------------------------------------------

#[tokio::test]
async fn evaluate_int_property_matches_float_eq_constraint() {
    // arrange: regression — serde_json::Number doesn't unify int/float
    // variants in PartialEq, so a stored constraint of `30.0` would silently
    // fail to match a property of `30`. The fix lives in `values_eq`.
    let app = TestApp::spawn().await;
    let api_key = app.seed_api_key().await;
    let body = json!({
        "key": "numeric_eq",
        "primaryMetric": "conv",
        "variants": [
            { "key": "control", "isControl": true, "config": {} },
            { "key": "treatment", "isControl": false, "config": { "feature": "on" } }
        ],
        "segments": [{
            "priority": 0,
            "rolloutPercent": 100,
            // Stored as Number::Float(30.0).
            "constraints": [
                { "property": "age", "operator": "EQ", "value": 30.0 }
            ],
            "distributions": [
                { "variantKey": "control", "percent": 0 },
                { "variantKey": "treatment", "percent": 100 }
            ]
        }]
    });
    app.running_experiment(&body).await;

    // act + assert: int property sent as Number::PosInt(30) — must still
    // match the float-typed segment constraint.
    let payload: Value = app
        .evaluate(
            &api_key,
            &json!({
                "experimentKey": "numeric_eq",
                "entityId": "u1",
                "properties": { "age": 30 }
            }),
        )
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(payload["variantKey"], "treatment");

    // act + assert: off-by-one still fails to match.
    let payload: Value = app
        .evaluate(
            &api_key,
            &json!({
                "experimentKey": "numeric_eq",
                "entityId": "u2",
                "properties": { "age": 31 }
            }),
        )
        .await
        .json()
        .await
        .unwrap();
    assert!(payload["variantKey"].is_null());
}

#[tokio::test]
async fn evaluate_int_property_matches_float_in_constraint() {
    // arrange: same regression as `_eq_`, applied to IN against a
    // float-typed array.
    let app = TestApp::spawn().await;
    let api_key = app.seed_api_key().await;
    let body = json!({
        "key": "numeric_in",
        "primaryMetric": "conv",
        "variants": [
            { "key": "control", "isControl": true, "config": {} },
            { "key": "treatment", "isControl": false, "config": {} }
        ],
        "segments": [{
            "priority": 0,
            "rolloutPercent": 100,
            "constraints": [
                { "property": "tier", "operator": "IN", "value": [1.0, 2.0, 3.0] }
            ],
            "distributions": [
                { "variantKey": "control", "percent": 0 },
                { "variantKey": "treatment", "percent": 100 }
            ]
        }]
    });
    app.running_experiment(&body).await;

    // act + assert: int property should land in [1.0, 2.0, 3.0] after the fix.
    let payload: Value = app
        .evaluate(
            &api_key,
            &json!({
                "experimentKey": "numeric_in",
                "entityId": "u1",
                "properties": { "tier": 2 }
            }),
        )
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(payload["variantKey"], "treatment");

    // act + assert: out-of-set value still fails.
    let payload: Value = app
        .evaluate(
            &api_key,
            &json!({
                "experimentKey": "numeric_in",
                "entityId": "u2",
                "properties": { "tier": 5 }
            }),
        )
        .await
        .json()
        .await
        .unwrap();
    assert!(payload["variantKey"].is_null());
}
