//! GET /admin/v1/experiments/{id}

use reqwest::StatusCode;
use serde_json::Value;

use super::created_id;
use crate::common::{valid_experiment_body, TestApp};

#[tokio::test]
async fn get_experiment_existing_ok() {
    // arrange
    let app = TestApp::spawn().await;
    let body = valid_experiment_body("full_repr");
    let id = created_id(&app, &body).await;

    // act
    let response = app.get_experiment(&id).await;

    // assert
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
async fn get_experiment_unknown_id_not_found() {
    // arrange
    let app = TestApp::spawn().await;

    // act
    let response = app
        .get_experiment("00000000-0000-0000-0000-000000000000")
        .await;

    // assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_experiment_other_tenant_not_found() {
    // arrange: user A creates an experiment, user B tries to read it.
    let app = TestApp::spawn().await;
    let id = created_id(&app, &valid_experiment_body("tenant_a")).await;
    let (_, other_token) = app.seed_other_user().await;

    // act
    let response = app
        .raw_client()
        .get(format!("{}/admin/v1/experiments/{id}", app.addr()))
        .bearer_auth(other_token)
        .send()
        .await
        .unwrap();

    // assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
