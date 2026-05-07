//! PATCH /admin/v1/experiments/{id}

use reqwest::StatusCode;
use serde_json::json;

use super::created_id;
use crate::common::{valid_experiment_body, TestApp};

#[tokio::test]
async fn description_succeeds() {
    // Arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("updatable")).await;

    // Act
    let response = app
        .patch_experiment(&id, &json!({ "description": "updated" }), None)
        .await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    let row: (Option<String>,) =
        sqlx::query_as("SELECT description FROM experiments WHERE experiment_id = $1")
            .bind(&id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row.0.as_deref(), Some("updated"));
}

#[tokio::test]
async fn clearing_description_sets_null() {
    // Arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("clearable")).await;

    // Act
    let response = app
        .patch_experiment(&id, &json!({ "description": null }), None)
        .await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    let row: (Option<String>,) =
        sqlx::query_as("SELECT description FROM experiments WHERE experiment_id = $1")
            .bind(&id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row.0, None);
}

#[tokio::test]
async fn primary_metric_change_blocked_when_running() {
    // Arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("running_pm")).await;
    assert_eq!(app.start_experiment(&id).await.status(), StatusCode::OK);

    // Act: try to change primary_metric on a running experiment
    let response = app
        .patch_experiment(&id, &json!({ "primaryMetric": "new_metric" }), None)
        .await;

    // Assert
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn variants_change_blocked_when_running() {
    let app = TestApp::spawn().await;
    let id = app.running_experiment(&valid_experiment_body("running_v")).await;

    let response = app
        .patch_experiment(
            &id,
            &json!({
                "variants": [
                    { "key": "control", "isControl": true, "config": {} },
                    { "key": "new_arm", "isControl": false, "config": {} }
                ]
            }),
            None,
        )
        .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn description_change_succeeds_when_running() {
    // Arrange
    let app = TestApp::spawn().await;
    let id = app
        .running_experiment(&valid_experiment_body("running_desc"))
        .await;

    // Act
    let response = app
        .patch_experiment(&id, &json!({ "description": "updated mid-flight" }), None)
        .await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    let row: (Option<String>,) =
        sqlx::query_as("SELECT description FROM experiments WHERE experiment_id = $1")
            .bind(&id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row.0.as_deref(), Some("updated mid-flight"));
}

#[tokio::test]
async fn rollout_increase_succeeds_when_running() {
    // Arrange: an experiment that starts at 25% rollout, then is started.
    let app = TestApp::spawn().await;
    let mut body = valid_experiment_body("ramp_up");
    body["segments"][0]["rolloutPercent"] = json!(25);
    let id = app.running_experiment(&body).await;

    // Act: bump rollout from 25 → 75. Same segment, same constraints, same
    // distributions — only rollout_percent changes.
    let response = app
        .patch_experiment(
            &id,
            &json!({
                "segments": [
                    {
                        "priority": 0,
                        "rolloutPercent": 75,
                        "constraints": [],
                        "distributions": [
                            { "variantKey": "control",   "percent": 50 },
                            { "variantKey": "treatment", "percent": 50 }
                        ]
                    }
                ]
            }),
            None,
        )
        .await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    let row: (String,) =
        sqlx::query_as("SELECT segments FROM experiments WHERE experiment_id = $1")
            .bind(&id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert!(row.0.contains("\"rolloutPercent\":75"));
}

#[tokio::test]
async fn rollout_decrease_rejected_when_running() {
    let app = TestApp::spawn().await;
    let mut body = valid_experiment_body("ramp_down");
    body["segments"][0]["rolloutPercent"] = json!(75);
    let id = app.running_experiment(&body).await;

    let response = app
        .patch_experiment(
            &id,
            &json!({
                "segments": [
                    {
                        "priority": 0,
                        "rolloutPercent": 10,
                        "constraints": [],
                        "distributions": [
                            { "variantKey": "control",   "percent": 50 },
                            { "variantKey": "treatment", "percent": 50 }
                        ]
                    }
                ]
            }),
            None,
        )
        .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn distribution_change_rejected_when_running() {
    let app = TestApp::spawn().await;
    let id = app
        .running_experiment(&valid_experiment_body("dist_change"))
        .await;

    let response = app
        .patch_experiment(
            &id,
            &json!({
                "segments": [
                    {
                        "priority": 0,
                        "rolloutPercent": 100,
                        "constraints": [],
                        "distributions": [
                            { "variantKey": "control",   "percent": 30 },
                            { "variantKey": "treatment", "percent": 70 }
                        ]
                    }
                ]
            }),
            None,
        )
        .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn segment_added_rejected_when_running() {
    let app = TestApp::spawn().await;
    let id = app
        .running_experiment(&valid_experiment_body("seg_add"))
        .await;

    let response = app
        .patch_experiment(
            &id,
            &json!({
                "segments": [
                    {
                        "priority": 0,
                        "rolloutPercent": 100,
                        "constraints": [],
                        "distributions": [
                            { "variantKey": "control",   "percent": 50 },
                            { "variantKey": "treatment", "percent": 50 }
                        ]
                    },
                    {
                        "priority": 1,
                        "rolloutPercent": 50,
                        "constraints": [],
                        "distributions": [
                            { "variantKey": "control",   "percent": 100 }
                        ]
                    }
                ]
            }),
            None,
        )
        .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn segments_change_rejected_when_stopped() {
    let app = TestApp::spawn().await;
    let id = app
        .running_experiment(&valid_experiment_body("ramp_stopped"))
        .await;
    assert_eq!(app.stop_experiment(&id).await.status(), StatusCode::OK);

    let response = app
        .patch_experiment(
            &id,
            &json!({
                "segments": [
                    {
                        "priority": 0,
                        "rolloutPercent": 100,
                        "constraints": [],
                        "distributions": [
                            { "variantKey": "control",   "percent": 50 },
                            { "variantKey": "treatment", "percent": 50 }
                        ]
                    }
                ]
            }),
            None,
        )
        .await;

    // A no-op segments value would succeed; we send the same shape with a
    // bumped rollout so it's a real change attempt.
    let mut body_higher = json!({
        "segments": [
            {
                "priority": 0,
                "rolloutPercent": 100,
                "constraints": [],
                "distributions": [
                    { "variantKey": "control",   "percent": 50 },
                    { "variantKey": "treatment", "percent": 50 }
                ]
            }
        ]
    });
    body_higher["segments"][0]["rolloutPercent"] = json!(50);
    let response2 = app.patch_experiment(&id, &body_higher, None).await;

    // Stopped is terminal — no segment edits permitted at all. The first
    // request happens to be unchanged (no-op success); the second is a real
    // diff and must 409.
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response2.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn description_change_succeeds_when_stopped() {
    let app = TestApp::spawn().await;
    let id = app
        .running_experiment(&valid_experiment_body("desc_stopped"))
        .await;
    assert_eq!(app.stop_experiment(&id).await.status(), StatusCode::OK);

    let response = app
        .patch_experiment(&id, &json!({ "description": "post-mortem note" }), None)
        .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn unchanged_full_payload_succeeds_when_running() {
    // Repro of the form-resubmit case: the UI sends primaryMetric (and
    // sometimes the full unchanged structure) on a running experiment when
    // only the description is being edited. Should be a 200 since nothing
    // structural actually changed.
    let app = TestApp::spawn().await;
    let id = app
        .running_experiment(&valid_experiment_body("noop_resubmit"))
        .await;

    let response = app
        .patch_experiment(
            &id,
            &json!({
                "description": "A test experiment",
                "primaryMetric": "conversion_rate"
            }),
            None,
        )
        .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn with_stale_if_match_returns_412() {
    // Arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("if_match")).await;

    // Act
    let response = app
        .patch_experiment(&id, &json!({ "description": "x" }), Some(0))
        .await;

    // Assert
    assert_eq!(response.status(), StatusCode::PRECONDITION_FAILED);
}

#[tokio::test]
async fn returns_404_for_unknown_id() {
    // Arrange
    let app = TestApp::spawn().await;

    // Act
    let response = app
        .patch_experiment(
            "00000000-0000-0000-0000-000000000000",
            &json!({ "description": "x" }),
            None,
        )
        .await;

    // Assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn with_empty_body_is_422() {
    // Arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("empty_patch")).await;

    // Act
    let response = app.patch_experiment(&id, &json!({}), None).await;

    // Assert
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
