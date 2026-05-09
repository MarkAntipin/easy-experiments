//! POST /admin/v1/experiments

use reqwest::StatusCode;
use serde_json::{json, Value};

use crate::common::{valid_experiment_body, TestApp};

#[tokio::test]
async fn create_experiment_valid_payload_ok() {
    // arrange
    let app = TestApp::spawn().await;
    let body = valid_experiment_body("checkout_button_color");

    // act
    let response = app.post_experiment(&body).await;

    // assert
    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: Value = response.json().await.unwrap();
    let experiment_id = payload["experimentId"]
        .as_str()
        .expect("experimentId in response")
        .to_string();
    assert!(!experiment_id.is_empty());

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
async fn create_experiment_invalid_payload_validation_error() {
    // arrange: distributions don't sum to 100
    let app = TestApp::spawn().await;
    let mut body = valid_experiment_body("bad");
    body["segments"][0]["distributions"][0]["percent"] = json!(10);

    // act
    let response = app.post_experiment(&body).await;

    // assert
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn create_experiment_duplicate_key_conflict() {
    // arrange
    let app = TestApp::spawn().await;
    let body = valid_experiment_body("dup_key");
    let first = app.post_experiment(&body).await;
    assert_eq!(first.status(), StatusCode::CREATED);

    // act
    let second = app.post_experiment(&body).await;

    // assert
    assert_eq!(second.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn create_experiment_same_key_across_tenants_ok() {
    // arrange: user A creates an experiment with a given key.
    let app = TestApp::spawn().await;
    let body = valid_experiment_body("shared_key");
    let first = app.post_experiment(&body).await;
    assert_eq!(first.status(), StatusCode::CREATED);

    let (other_user, other_token) = app.seed_other_user().await;

    // act: user B (different company) creates an experiment with the same key.
    let second = app
        .raw_client()
        .post(format!("{}/admin/v1/experiments", app.addr()))
        .bearer_auth(&other_token)
        .json(&body)
        .send()
        .await
        .unwrap();

    // assert: both creations succeed and each row is scoped to its own company.
    assert_eq!(second.status(), StatusCode::CREATED);

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM experiments WHERE key = $1 AND company_id = $2")
            .bind("shared_key")
            .bind(&other_user.company_id)
            .fetch_one(&app.pool)
            .await
            .expect("count for other tenant");
    assert_eq!(count.0, 1);

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM experiments WHERE key = $1 AND company_id = $2")
            .bind("shared_key")
            .bind(&app.user.company_id)
            .fetch_one(&app.pool)
            .await
            .expect("count for primary tenant");
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn create_experiment_missing_jwt_unauthorized() {
    // arrange
    let app = TestApp::spawn().await;
    let body = valid_experiment_body("needs_auth");

    // act: no bearer token
    let response = app
        .raw_client()
        .post(format!("{}/admin/v1/experiments", app.addr()))
        .json(&body)
        .send()
        .await
        .unwrap();

    // assert
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
