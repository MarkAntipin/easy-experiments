//! POST /admin/v1/auth/login

use easy_experiments::models::UserRole;
use reqwest::StatusCode;
use serde_json::Value;

use crate::common::TestApp;

#[tokio::test]
async fn password_login_with_valid_credentials_returns_token() {
    let app = TestApp::spawn().await;
    app.seed_password_user("alice@acme.test", "correct horse battery", UserRole::Member)
        .await;

    let resp = app
        .password_login("alice@acme.test", "correct horse battery")
        .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(body["token"].as_str().unwrap().len() > 20);
    assert_eq!(body["user"]["email"], "alice@acme.test");
    assert_eq!(body["user"]["role"], "member");
}

#[tokio::test]
async fn password_login_with_wrong_password_unauthorized() {
    let app = TestApp::spawn().await;
    app.seed_password_user("alice@acme.test", "secret-one", UserRole::Member)
        .await;

    let resp = app.password_login("alice@acme.test", "secret-two").await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn password_login_unknown_email_unauthorized() {
    // No leakage between "wrong password" and "no such user".
    let app = TestApp::spawn().await;
    let resp = app.password_login("nobody@acme.test", "whatever").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn password_login_on_google_only_account_unauthorized() {
    // The seeded acme owner is google-only (google_sub set, password_hash NULL).
    // Trying to log in via password against that row must be rejected — and
    // with the same generic 401 the bad-password branch returns, not a hint.
    let app = TestApp::spawn().await;
    let resp = app.password_login("owner@acme.test", "anything").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn password_login_minted_jwt_works_against_admin_endpoint() {
    // End-to-end loop: log in, then use the returned JWT to hit a protected
    // admin route. Verifies the JWT is properly bound to the user/company.
    let app = TestApp::spawn().await;
    app.seed_password_user("alice@acme.test", "good-password", UserRole::Admin)
        .await;

    let login = app.password_login("alice@acme.test", "good-password").await;
    assert_eq!(login.status(), StatusCode::OK);
    let body: Value = login.json().await.unwrap();
    let token = body["token"].as_str().unwrap().to_string();

    let resp = app
        .raw_client()
        .get(format!("{}/admin/v1/users", app.addr()))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn password_login_empty_body_unauthorized() {
    let app = TestApp::spawn().await;
    let resp = app.password_login("", "").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
