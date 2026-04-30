//! GET /admin/v1/experiments/{id}

use reqwest::StatusCode;
use serde_json::Value;

use super::created_id;
use crate::common::{valid_experiment_body, TestApp};

#[tokio::test]
async fn returns_full_representation() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = valid_experiment_body("full_repr");
    let id = created_id(&app, &body).await;

    // Act
    let response = app.get_experiment(&id).await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    let payload: Value = response.json().await.unwrap();
    assert_eq!(payload["experimentId"], id);
    assert_eq!(payload["key"], "full_repr");
    assert_eq!(payload["status"], "draft");
    assert_eq!(payload["primaryMetric"], "conversion_rate");
    assert_eq!(payload["variants"].as_array().unwrap().len(), 2);
    assert_eq!(payload["segments"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn returns_404_for_unknown_id() {
    // Arrange
    let app = TestApp::spawn().await;

    // Act
    let response = app.get_experiment("does-not-exist").await;

    // Assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn does_not_leak_across_tenants() {
    // Arrange: user A creates an experiment, user B tries to read it.
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("tenant_a")).await;
    let (_, other_token) = app.seed_other_user().await;

    // Act
    let response = app
        .raw_client()
        .get(format!("{}/admin/v1/experiments/{id}", app.addr()))
        .bearer_auth(other_token)
        .send()
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
