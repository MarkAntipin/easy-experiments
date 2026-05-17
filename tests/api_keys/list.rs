//! GET /admin/v1/api-keys

use reqwest::StatusCode;
use serde_json::Value;

use super::created_id;
use crate::common::TestApp;

#[tokio::test]
async fn list_api_keys_empty_ok() {
    // arrange
    let app = TestApp::spawn().await;

    // act
    let response = app.list_api_keys().await;

    // assert
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    let items = body["items"].as_array().expect("items array");
    assert!(items.is_empty());
}

#[tokio::test]
async fn list_api_keys_multiple_ok() {
    // arrange
    let app = TestApp::spawn().await;
    created_id(&app, "first").await;
    created_id(&app, "second").await;

    // act
    let response = app.list_api_keys().await;

    // assert
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    let items = body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 2);
    let names: Vec<&str> = items.iter().map(|i| i["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"first"));
    assert!(names.contains(&"second"));
    // Plaintext key is never returned by list — only by create.
    for item in items {
        assert!(item.get("key").is_none());
        assert!(item["apiKeyId"].as_str().is_some());
        assert!(item["keyPrefix"].as_str().is_some());
        assert!(item["createdAt"].as_i64().is_some());
    }
}

#[tokio::test]
async fn list_api_keys_excludes_other_tenants() {
    // arrange: tenant A has a key, tenant B has none.
    let app = TestApp::spawn().await;
    created_id(&app, "tenant_a_key").await;
    let (_, other_token) = app.seed_other_user().await;

    // act: list as tenant B
    let response = app
        .raw_client()
        .get(format!("{}/admin/v1/api-keys", app.addr()))
        .bearer_auth(other_token)
        .send()
        .await
        .unwrap();

    // assert
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    let items = body["items"].as_array().expect("items array");
    assert!(items.is_empty());
}

#[tokio::test]
async fn list_api_keys_missing_jwt_unauthorized() {
    // arrange
    let app = TestApp::spawn().await;

    // act: no bearer token
    let response = app
        .raw_client()
        .get(format!("{}/admin/v1/api-keys", app.addr()))
        .send()
        .await
        .unwrap();

    // assert
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
