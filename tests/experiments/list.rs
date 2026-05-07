//! GET /admin/v1/experiments

use reqwest::StatusCode;
use serde_json::Value;

use super::created_id;
use crate::common::{valid_experiment_body, TestApp};

#[tokio::test]
async fn returns_all_for_company() {
    // Arrange
    let app = TestApp::spawn().await;
    created_id(&app, &valid_experiment_body("exp_a")).await;
    created_id(&app, &valid_experiment_body("exp_b")).await;

    // Act
    let response = app.list_experiments(None).await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    let items = body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 2);
    let keys: Vec<&str> = items.iter().map(|i| i["key"].as_str().unwrap()).collect();
    assert!(keys.contains(&"exp_a"));
    assert!(keys.contains(&"exp_b"));
}

#[tokio::test]
async fn filters_by_status() {
    // Arrange: one draft, one running.
    let app = TestApp::spawn().await;
    created_id(&app, &valid_experiment_body("stays_draft")).await;
    let running_id = created_id(&app, &valid_experiment_body("will_run")).await;
    assert_eq!(
        app.start_experiment(&running_id).await.status(),
        StatusCode::OK
    );

    // Act
    let response = app.list_experiments(Some("running")).await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    let items = body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["experimentId"], running_id);
    assert_eq!(items[0]["status"], "running");
}

#[tokio::test]
async fn rejects_deleted_filter_with_422() {
    // Arrange
    let app = TestApp::spawn().await;

    // Act
    let response = app.list_experiments(Some("deleted")).await;

    // Assert
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
