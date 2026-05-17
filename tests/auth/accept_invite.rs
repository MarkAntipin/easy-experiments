//! POST /admin/v1/auth/accept-invite

use reqwest::StatusCode;
use serde_json::{json, Value};

use crate::common::TestApp;

/// Drive the existing admin invite endpoint and pluck out the one-time token
/// the server hands back. This is the realistic "admin invites teammate"
/// path; reuse it across the accept-invite scenarios so we exercise the same
/// shape the UI sees.
async fn invite_and_get_token(app: &TestApp, email: &str) -> String {
    let resp = app.post_user(&json!({ "email": email })).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "invite precondition");
    let body: Value = resp.json().await.unwrap();
    body["inviteToken"]
        .as_str()
        .expect("inviteToken in invite response when password provider is enabled")
        .to_string()
}

#[tokio::test]
async fn invite_response_contains_token_and_url() {
    // Surface check: with the password provider enabled, the response from
    // `POST /admin/v1/users` exposes the one-time invite token plus a
    // copy-paste link. UI relies on these fields existing.
    let app = TestApp::spawn().await;
    let resp = app.post_user(&json!({ "email": "alice@acme.test" })).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = resp.json().await.unwrap();
    assert!(
        body["inviteToken"].as_str().is_some(),
        "inviteToken present"
    );
    assert!(body["inviteUrl"].as_str().is_some(), "inviteUrl present");
    assert!(
        body["inviteExpiresAt"].as_i64().is_some(),
        "inviteExpiresAt present"
    );
}

#[tokio::test]
async fn accept_invite_happy_path_sets_password_and_returns_jwt() {
    let app = TestApp::spawn().await;
    let token = invite_and_get_token(&app, "alice@acme.test").await;

    let resp = app.accept_invite(&token, "new-strong-password").await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(body["token"].as_str().unwrap().len() > 20);
    assert_eq!(body["user"]["email"], "alice@acme.test");

    // The row should now carry a password_hash and have its invite token cleared.
    let (password_hash, invite_hash, expires): (Option<String>, Option<String>, Option<i64>) =
        sqlx::query_as(
            "SELECT password_hash, invite_token_hash, invite_token_expires_at
             FROM users WHERE email = $1",
        )
        .bind("alice@acme.test")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert!(password_hash.is_some(), "password_hash set");
    assert!(invite_hash.is_none(), "invite_token_hash cleared");
    assert!(expires.is_none(), "invite_token_expires_at cleared");
}

#[tokio::test]
async fn accept_invite_then_password_login_succeeds() {
    let app = TestApp::spawn().await;
    let token = invite_and_get_token(&app, "alice@acme.test").await;
    let accept = app.accept_invite(&token, "loops-around").await;
    assert_eq!(accept.status(), StatusCode::OK);

    let login = app.password_login("alice@acme.test", "loops-around").await;
    assert_eq!(login.status(), StatusCode::OK);
}

#[tokio::test]
async fn accept_invite_with_unknown_token_unauthorized() {
    let app = TestApp::spawn().await;
    let resp = app
        .accept_invite("totally-fake-token", "doesnt-matter-yet-strong")
        .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn accept_invite_second_time_is_rejected() {
    // Tokens are one-time: hash cleared on first accept. A replay must fail.
    let app = TestApp::spawn().await;
    let token = invite_and_get_token(&app, "alice@acme.test").await;
    let first = app.accept_invite(&token, "first-password").await;
    assert_eq!(first.status(), StatusCode::OK);

    let second = app.accept_invite(&token, "second-password").await;
    assert_eq!(second.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn accept_invite_with_expired_token_unauthorized() {
    let app = TestApp::spawn().await;
    let token = invite_and_get_token(&app, "alice@acme.test").await;

    // Backdate the expiry directly so we don't have to wait 14 days.
    sqlx::query("UPDATE users SET invite_token_expires_at = 1 WHERE email = $1")
        .bind("alice@acme.test")
        .execute(&app.pool)
        .await
        .unwrap();

    let resp = app.accept_invite(&token, "doesnt-matter-yet-strong").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn accept_invite_with_short_password_validation_error() {
    let app = TestApp::spawn().await;
    let token = invite_and_get_token(&app, "alice@acme.test").await;

    let resp = app.accept_invite(&token, "short").await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn accept_invite_with_empty_token_validation_error() {
    let app = TestApp::spawn().await;
    let resp = app.accept_invite("", "long-enough-password").await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
