//! POST /admin/v1/experiments/{id}/start and /stop

use reqwest::StatusCode;

use super::created_id;
use crate::common::{valid_experiment_body, TestApp};

#[tokio::test]
async fn start_experiment_draft_ok() {
    // arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("to_start")).await;

    // act
    let response = app.start_experiment(&id).await;

    // assert
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
async fn start_experiment_already_running_conflict() {
    // arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("already_running")).await;
    assert_eq!(app.start_experiment(&id).await.status(), StatusCode::OK);

    // act
    let response = app.start_experiment(&id).await;

    // assert
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn stop_experiment_running_ok() {
    // arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("to_stop")).await;
    assert_eq!(app.start_experiment(&id).await.status(), StatusCode::OK);

    // act
    let response = app.stop_experiment(&id).await;

    // assert
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
async fn stop_experiment_draft_conflict() {
    // arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("still_draft")).await;

    // act
    let response = app.stop_experiment(&id).await;

    // assert
    assert_eq!(response.status(), StatusCode::CONFLICT);
}
