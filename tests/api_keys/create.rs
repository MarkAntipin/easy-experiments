//! POST /admin/v1/api-keys

use reqwest::StatusCode;
use serde_json::{json, Value};

use crate::common::TestApp;

#[tokio::test]
async fn create_api_key_valid_payload_ok() {
    // arrange
    let app = TestApp::spawn().await;

    // act
    let response = app.post_api_key(&json!({ "name": "production" })).await;

    // assert
    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: Value = response.json().await.unwrap();
    let api_key_id = payload["apiKeyId"]
        .as_str()
        .expect("apiKeyId in response")
        .to_string();
    assert!(!api_key_id.is_empty());
    assert_eq!(payload["name"], "production");
    let key = payload["key"].as_str().expect("plaintext key in response");
    assert!(key.starts_with("eek-"));
    let prefix = payload["keyPrefix"]
        .as_str()
        .expect("keyPrefix in response");
    assert!(key.starts_with(prefix));

    let row: (String, String, String) =
        sqlx::query_as("SELECT name, key_prefix, company_id FROM api_keys WHERE api_key_id = $1")
            .bind(&api_key_id)
            .fetch_one(&app.pool)
            .await
            .expect("api key persisted");
    assert_eq!(row.0, "production");
    assert_eq!(row.1, prefix);
    assert_eq!(row.2, app.user.company_id);
}

#[tokio::test]
async fn create_api_key_empty_name_validation_error() {
    // arrange
    let app = TestApp::spawn().await;

    // act
    let response = app.post_api_key(&json!({ "name": "" })).await;

    // assert
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn create_api_key_whitespace_padded_name_validation_error() {
    // arrange: leading/trailing whitespace is rejected to keep names canonical.
    let app = TestApp::spawn().await;

    // act
    let response = app.post_api_key(&json!({ "name": "  prod  " })).await;

    // assert
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn create_api_key_too_long_name_validation_error() {
    // arrange: 129 bytes > 128-byte cap.
    let app = TestApp::spawn().await;
    let too_long = "a".repeat(129);

    // act
    let response = app.post_api_key(&json!({ "name": too_long })).await;

    // assert
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn create_api_key_duplicate_name_conflict() {
    // arrange
    let app = TestApp::spawn().await;
    let body = json!({ "name": "dup_name" });
    let first = app.post_api_key(&body).await;
    assert_eq!(first.status(), StatusCode::CREATED);

    // act
    let second = app.post_api_key(&body).await;

    // assert
    assert_eq!(second.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn create_api_key_same_name_across_tenants_ok() {
    // arrange: user A creates a key with a given name.
    let app = TestApp::spawn().await;
    let body = json!({ "name": "shared_name" });
    let first = app.post_api_key(&body).await;
    assert_eq!(first.status(), StatusCode::CREATED);

    let (other_user, other_token) = app.seed_other_user().await;

    // act: user B (different company) creates a key with the same name.
    let second = app
        .raw_client()
        .post(format!("{}/admin/v1/api-keys", app.addr()))
        .bearer_auth(&other_token)
        .json(&body)
        .send()
        .await
        .unwrap();

    // assert: both creations succeed and each row is scoped to its own company.
    assert_eq!(second.status(), StatusCode::CREATED);

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM api_keys WHERE name = $1 AND company_id = $2")
            .bind("shared_name")
            .bind(&other_user.company_id)
            .fetch_one(&app.pool)
            .await
            .expect("count for other tenant");
    assert_eq!(count.0, 1);

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM api_keys WHERE name = $1 AND company_id = $2")
            .bind("shared_name")
            .bind(&app.user.company_id)
            .fetch_one(&app.pool)
            .await
            .expect("count for primary tenant");
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn create_api_key_missing_jwt_unauthorized() {
    // arrange
    let app = TestApp::spawn().await;
    let body = json!({ "name": "needs_auth" });

    // act: no bearer token
    let response = app
        .raw_client()
        .post(format!("{}/admin/v1/api-keys", app.addr()))
        .json(&body)
        .send()
        .await
        .unwrap();

    // assert
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
