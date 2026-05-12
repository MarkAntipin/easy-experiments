//! POST /admin/v1/users

use reqwest::StatusCode;
use serde_json::{json, Value};

use crate::common::TestApp;

#[tokio::test]
async fn invite_user_valid_email_ok() {
    // arrange
    let app = TestApp::spawn().await;

    // act
    let response = app.post_user(&json!({ "email": "alice@acme.test" })).await;

    // assert
    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = response.json().await.unwrap();
    let user_id = body["userId"]
        .as_str()
        .expect("userId in response")
        .to_string();
    assert!(!user_id.is_empty());
    assert_eq!(body["email"], "alice@acme.test");
    assert_eq!(body["status"], "pending");

    let row: (String, String, Option<String>) = sqlx::query_as(
        "SELECT email, company_id, google_sub FROM users WHERE user_id = $1",
    )
    .bind(&user_id)
    .fetch_one(&app.pool)
    .await
    .expect("invited user persisted");
    assert_eq!(row.0, "alice@acme.test");
    assert_eq!(row.1, app.user.company_id);
    assert!(
        row.2.is_none(),
        "google_sub should be NULL for a pending invite"
    );
}

#[tokio::test]
async fn invite_user_lowercases_and_trims_email() {
    // arrange: emails are normalized so casing typos don't create duplicates.
    let app = TestApp::spawn().await;

    // act
    let response = app
        .post_user(&json!({ "email": "  Bob@Acme.Test  " }))
        .await;

    // assert
    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["email"], "bob@acme.test");
}

#[tokio::test]
async fn invite_user_empty_email_validation_error() {
    let app = TestApp::spawn().await;
    let response = app.post_user(&json!({ "email": "" })).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn invite_user_no_at_sign_validation_error() {
    let app = TestApp::spawn().await;
    let response = app.post_user(&json!({ "email": "not-an-email" })).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn invite_user_missing_domain_dot_validation_error() {
    let app = TestApp::spawn().await;
    let response = app.post_user(&json!({ "email": "bob@nodot" })).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn invite_user_too_long_validation_error() {
    let app = TestApp::spawn().await;
    let too_long = format!("{}@acme.test", "a".repeat(260));
    let response = app.post_user(&json!({ "email": too_long })).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn invite_user_duplicate_email_conflict() {
    // arrange: same email invited twice in the same company.
    let app = TestApp::spawn().await;
    let body = json!({ "email": "dup@acme.test" });
    let first = app.post_user(&body).await;
    assert_eq!(first.status(), StatusCode::CREATED);

    // act
    let second = app.post_user(&body).await;

    // assert
    assert_eq!(second.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn invite_user_email_collides_across_tenants_conflict() {
    // Email is globally unique in `users`, so the same address can only live
    // in one company. Tenant B trying to invite an address tenant A already
    // owns should 409 — surfaces clearly to the admin instead of silently
    // succeeding or 500'ing on a UNIQUE violation.
    let app = TestApp::spawn().await;
    let body = json!({ "email": "shared@acme.test" });
    let first = app.post_user(&body).await;
    assert_eq!(first.status(), StatusCode::CREATED);

    let (_, other_token) = app.seed_other_user().await;

    let response = app
        .raw_client()
        .post(format!("{}/admin/v1/users", app.addr()))
        .bearer_auth(other_token)
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn invite_user_owner_email_conflict() {
    // The seeded owner already exists under owner@acme.test; re-inviting
    // their email must 409, not silently overwrite or 500.
    let app = TestApp::spawn().await;
    let response = app
        .post_user(&json!({ "email": "owner@acme.test" }))
        .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn invite_user_missing_jwt_unauthorized() {
    let app = TestApp::spawn().await;
    let response = app
        .raw_client()
        .post(format!("{}/admin/v1/users", app.addr()))
        .json(&json!({ "email": "needs_auth@acme.test" }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
