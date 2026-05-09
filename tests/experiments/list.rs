//! GET /admin/v1/experiments

use reqwest::StatusCode;
use serde_json::Value;

use super::created_id;
use crate::common::{valid_experiment_body, TestApp};

#[tokio::test]
async fn list_experiments_no_filter_ok() {
    // arrange
    let app = TestApp::spawn().await;
    created_id(&app, &valid_experiment_body("exp_a")).await;
    created_id(&app, &valid_experiment_body("exp_b")).await;

    // act
    let response = app.list_experiments(None).await;

    // assert
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    let items = body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 2);
    let keys: Vec<&str> = items.iter().map(|i| i["key"].as_str().unwrap()).collect();
    assert!(keys.contains(&"exp_a"));
    assert!(keys.contains(&"exp_b"));
}

#[tokio::test]
async fn list_experiments_status_filter_ok() {
    // arrange: one draft, one running.
    let app = TestApp::spawn().await;
    created_id(&app, &valid_experiment_body("stays_draft")).await;
    let running_id = created_id(&app, &valid_experiment_body("will_run")).await;
    assert_eq!(
        app.start_experiment(&running_id).await.status(),
        StatusCode::OK
    );

    // act
    let response = app.list_experiments(Some("running")).await;

    // assert
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    let items = body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["experimentId"], running_id);
    assert_eq!(items[0]["status"], "running");
}

#[tokio::test]
async fn list_experiments_deleted_filter_validation_error() {
    // arrange
    let app = TestApp::spawn().await;

    // act
    let response = app.list_experiments(Some("deleted")).await;

    // assert
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
