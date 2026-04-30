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
    let response = app.delete_experiment("does-not-exist").await;

    // Assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
