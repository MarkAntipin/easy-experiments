//! POST /admin/v1/experiments

use reqwest::StatusCode;
use serde_json::{json, Value};

use crate::common::{valid_experiment_body, TestApp};

#[tokio::test]
async fn returns_201_and_persists() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = valid_experiment_body("checkout_button_color");

    // Act
    let response = app.post_experiment(&body).await;

    // Assert
    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: Value = response.json().await.unwrap();
    let experiment_id = payload["experimentId"]
        .as_str()
        .expect("experimentId in response")
        .to_string();
    assert!(!experiment_id.is_empty());
    assert_eq!(payload["message"], "Experiment created");

    let row: (String, String) = sqlx::query_as(
        "SELECT key, status FROM experiments WHERE experiment_id = $1 AND company_id = $2",
    )
    .bind(&experiment_id)
    .bind(&app.user.company_id)
    .fetch_one(&app.pool)
    .await
    .expect("experiment persisted");
    assert_eq!(row.0, "checkout_button_color");
    assert_eq!(row.1, "draft");
}

#[tokio::test]
async fn rejects_invalid_payload_with_422() {
    // Arrange: distributions don't sum to 100
    let app = TestApp::spawn().await;
    let mut body = valid_experiment_body("bad");
    body["segments"][0]["distributions"][0]["percent"] = json!(10);

    // Act
    let response = app.post_experiment(&body).await;

    // Assert
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn rejects_duplicate_key_with_409() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = valid_experiment_body("dup_key");
    let first = app.post_experiment(&body).await;
    assert_eq!(first.status(), StatusCode::CREATED);

    // Act
    let second = app.post_experiment(&body).await;

    // Assert
    assert_eq!(second.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn requires_jwt() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = valid_experiment_body("needs_auth");

    // Act: no bearer token
    let response = app
        .raw_client()
        .post(format!("{}/admin/v1/experiments", app.addr()))
        .json(&body)
        .send()
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
