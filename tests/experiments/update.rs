//! PATCH /admin/v1/experiments/{id}

use reqwest::StatusCode;
use serde_json::json;

use super::created_id;
use crate::common::{valid_experiment_body, TestApp};

#[tokio::test]
async fn update_experiment_description_ok() {
    // arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("updatable")).await;

    // act
    let response = app
        .patch_experiment(&id, &json!({ "description": "updated" }), None)
        .await;

    // assert
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
async fn update_experiment_null_description_ok() {
    // arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("clearable")).await;

    // act
    let response = app
        .patch_experiment(&id, &json!({ "description": null }), None)
        .await;

    // assert
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
async fn update_experiment_primary_metric_when_running_conflict() {
    // arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("running_pm")).await;
    assert_eq!(app.start_experiment(&id).await.status(), StatusCode::OK);

    // act: try to change primary_metric on a running experiment
    let response = app
        .patch_experiment(&id, &json!({ "primaryMetric": "new_metric" }), None)
        .await;

    // assert
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn update_experiment_variants_when_running_conflict() {
    let app = TestApp::spawn().await;
    let id = app
        .running_experiment(&valid_experiment_body("running_v"))
        .await;

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
async fn update_experiment_description_when_running_ok() {
    // arrange
    let app = TestApp::spawn().await;
    let id = app
        .running_experiment(&valid_experiment_body("running_desc"))
        .await;

    // act
    let response = app
        .patch_experiment(&id, &json!({ "description": "updated mid-flight" }), None)
        .await;

    // assert
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
async fn update_experiment_rollout_increase_when_running_ok() {
    // arrange: an experiment that starts at 25% rollout, then is started.
    let app = TestApp::spawn().await;
    let mut body = valid_experiment_body("ramp_up");
    body["segments"][0]["rolloutPercent"] = json!(25);
    let id = app.running_experiment(&body).await;

    // act: bump rollout from 25 → 75. Same segment, same constraints, same
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

    // assert
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
async fn update_experiment_rollout_decrease_when_running_conflict() {
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
async fn update_experiment_distribution_when_running_conflict() {
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
async fn update_experiment_segment_added_when_running_conflict() {
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
async fn update_experiment_segments_when_stopped_conflict() {
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
async fn update_experiment_description_when_stopped_ok() {
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
async fn update_experiment_unchanged_payload_when_running_ok() {
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
async fn update_experiment_stale_if_match_precondition_failed() {
    // arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("if_match")).await;

    // act
    let response = app
        .patch_experiment(&id, &json!({ "description": "x" }), Some(0))
        .await;

    // assert
    assert_eq!(response.status(), StatusCode::PRECONDITION_FAILED);
}

#[tokio::test]
async fn update_experiment_unknown_id_not_found() {
    // arrange
    let app = TestApp::spawn().await;

    // act
    let response = app
        .patch_experiment(
            "00000000-0000-0000-0000-000000000000",
            &json!({ "description": "x" }),
            None,
        )
        .await;

    // assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_experiment_empty_body_validation_error() {
    // arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("empty_patch")).await;

    // act
    let response = app.patch_experiment(&id, &json!({}), None).await;

    // assert
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
