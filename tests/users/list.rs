//! GET /admin/v1/users

use reqwest::StatusCode;
use serde_json::Value;

use super::invited_id;
use crate::common::TestApp;

#[tokio::test]
async fn list_users_shows_only_owner_when_no_invites() {
    // arrange: the seeded owner is the only user in their company.
    let app = TestApp::spawn().await;

    // act
    let response = app.list_users().await;

    // assert
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    let items = body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["userId"], app.user.user_id);
    assert_eq!(items[0]["status"], "active");
    assert_eq!(items[0]["email"], app.user.email);
}

#[tokio::test]
async fn list_users_includes_pending_and_active() {
    // arrange
    let app = TestApp::spawn().await;
    invited_id(&app, "alice@acme.test").await;
    invited_id(&app, "bob@acme.test").await;

    // act
    let response = app.list_users().await;

    // assert
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    let items = body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 3);

    let by_email: std::collections::HashMap<&str, &Value> = items
        .iter()
        .map(|i| (i["email"].as_str().unwrap(), i))
        .collect();

    assert_eq!(by_email[&app.user.email.as_str()]["status"], "active");
    assert_eq!(by_email["alice@acme.test"]["status"], "pending");
    assert_eq!(by_email["bob@acme.test"]["status"], "pending");
}

#[tokio::test]
async fn list_users_excludes_other_tenants() {
    // arrange: each tenant only sees their own members.
    let app = TestApp::spawn().await;
    invited_id(&app, "tenant_a_invitee@acme.test").await;
    let (_, other_token) = app.seed_other_user().await;

    // act: list as tenant B
    let response = app
        .raw_client()
        .get(format!("{}/admin/v1/users", app.addr()))
        .bearer_auth(other_token)
        .send()
        .await
        .unwrap();

    // assert: tenant B only sees their own owner row.
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    let items = body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1);
    assert_ne!(items[0]["email"], "tenant_a_invitee@acme.test");
}

#[tokio::test]
async fn list_users_missing_jwt_unauthorized() {
    let app = TestApp::spawn().await;
    let response = app
        .raw_client()
        .get(format!("{}/admin/v1/users", app.addr()))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
