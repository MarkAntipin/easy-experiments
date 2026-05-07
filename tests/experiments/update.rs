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
async fn structural_change_blocked_when_running() {
    // Arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("running_exp")).await;
    assert_eq!(app.start_experiment(&id).await.status(), StatusCode::OK);

    // Act: try to change primary_metric on a running experiment
    let response = app
        .patch_experiment(&id, &json!({ "primaryMetric": "new_metric" }), None)
        .await;

    // Assert
    assert_eq!(response.status(), StatusCode::CONFLICT);
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
