//! POST /admin/v1/experiments/{id}/start and /stop

use reqwest::StatusCode;

use super::created_id;
use crate::common::{valid_experiment_body, TestApp};

#[tokio::test]
async fn start_transitions_draft_to_running() {
    // Arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("to_start")).await;

    // Act
    let response = app.start_experiment(&id).await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    let row: (String, Option<i64>) =
        sqlx::query_as("SELECT status, started_at FROM experiments WHERE experiment_id = $1")
            .bind(&id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row.0, "running");
    assert!(row.1.is_some());
}

#[tokio::test]
async fn start_already_running_returns_409() {
    // Arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("already_running")).await;
    assert_eq!(app.start_experiment(&id).await.status(), StatusCode::OK);

    // Act
    let response = app.start_experiment(&id).await;

    // Assert
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn stop_running_transitions_to_stopped() {
    // Arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("to_stop")).await;
    assert_eq!(app.start_experiment(&id).await.status(), StatusCode::OK);

    // Act
    let response = app.stop_experiment(&id).await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    let row: (String, Option<i64>) =
        sqlx::query_as("SELECT status, stopped_at FROM experiments WHERE experiment_id = $1")
            .bind(&id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row.0, "stopped");
    assert!(row.1.is_some());
}

#[tokio::test]
async fn stop_draft_returns_409() {
    // Arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("still_draft")).await;

    // Act
    let response = app.stop_experiment(&id).await;

    // Assert
    assert_eq!(response.status(), StatusCode::CONFLICT);
}
