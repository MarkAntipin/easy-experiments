//! DELETE /admin/v1/experiments/{id}

use reqwest::StatusCode;

use super::created_id;
use crate::common::{valid_experiment_body, TestApp};

#[tokio::test]
async fn draft_soft_deletes_row() {
    // Arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("to_delete")).await;

    // Act
    let response = app.delete_experiment(&id).await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    // Subsequent GET (which filters deleted) should 404.
    assert_eq!(
        app.get_experiment(&id).await.status(),
        StatusCode::NOT_FOUND
    );
    // Row is still there, status = 'deleted'.
    let row: (String,) =
        sqlx::query_as("SELECT status FROM experiments WHERE experiment_id = $1")
            .bind(&id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row.0, "deleted");
}

#[tokio::test]
async fn running_returns_409() {
    // Arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("running_delete")).await;
    assert_eq!(app.start_experiment(&id).await.status(), StatusCode::OK);

    // Act
    let response = app.delete_experiment(&id).await;

    // Assert
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn unknown_returns_404() {
    // Arrange
    let app = TestApp::spawn().await;

    // Act
    let response = app
        .delete_experiment("00000000-0000-0000-0000-000000000000")
        .await;

    // Assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn malformed_id_returns_422() {
    // Arrange
    let app = TestApp::spawn().await;

    // Act
    let response = app.delete_experiment("not-a-uuid").await;

    // Assert
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn stopped_soft_deletes_row() {
    // Arrange
    let app = TestApp::spawn().await;
    let id = app
        .running_experiment(&valid_experiment_body("stopped_to_delete"))
        .await;
    assert_eq!(app.stop_experiment(&id).await.status(), StatusCode::OK);

    // Act
    let response = app.delete_experiment(&id).await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    let row: (String,) =
        sqlx::query_as("SELECT status FROM experiments WHERE experiment_id = $1")
            .bind(&id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row.0, "deleted");
}

#[tokio::test]
async fn does_not_leak_across_tenants() {
    // Arrange: user A creates an experiment, user B tries to delete it.
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("tenant_a")).await;
    let (_, other_token) = app.seed_other_user().await;

    // Act
    let response = app
        .raw_client()
        .delete(format!("{}/admin/v1/experiments/{id}", app.addr()))
        .bearer_auth(other_token)
        .send()
        .await
        .unwrap();

    // Assert: 404, and the row is untouched (still 'draft', not 'deleted').
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let row: (String,) =
        sqlx::query_as("SELECT status FROM experiments WHERE experiment_id = $1")
            .bind(&id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row.0, "draft");
}

#[tokio::test]
async fn repeat_delete_returns_404() {
    // Arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("delete_twice")).await;
    assert_eq!(app.delete_experiment(&id).await.status(), StatusCode::OK);

    // Act
    let response = app.delete_experiment(&id).await;

    // Assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
