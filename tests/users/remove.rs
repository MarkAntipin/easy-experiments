//! DELETE /admin/v1/users/{id}

use easy_experiments::models::ExperimentsDB;
use easy_experiments::services::auth::provision_and_mint;
use easy_experiments::services::google_auth::GoogleTokenInfo;
use reqwest::StatusCode;

use super::invited_id;
use crate::common::TestApp;

const TEST_JWT_SECRET: &str = "integration-test-jwt-secret";

fn token_info(sub: &str, email: &str) -> GoogleTokenInfo {
    GoogleTokenInfo {
        sub: sub.to_string(),
        email: email.to_string(),
        email_verified: true,
        name: String::new(),
        picture: String::new(),
    }
}

#[tokio::test]
async fn remove_user_pending_no_content() {
    // arrange
    let app = TestApp::spawn().await;
    let id = invited_id(&app, "pending@acme.test").await;

    // act
    let response = app.delete_user(&id).await;

    // assert
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE user_id = $1")
            .bind(&id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(count.0, 0);
}

#[tokio::test]
async fn remove_user_self_forbidden() {
    // The owner cannot remove themselves — would lock the company out and
    // break the JWT they're currently holding.
    let app = TestApp::spawn().await;

    // act
    let response = app.delete_user(&app.user.user_id).await;

    // assert
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE user_id = $1")
            .bind(&app.user.user_id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(count.0, 1, "self row should survive a self-delete attempt");
}

#[tokio::test]
async fn remove_user_unknown_id_no_content() {
    // Idempotent: removing an id that doesn't exist returns 204.
    let app = TestApp::spawn().await;
    let response = app
        .delete_user("00000000-0000-0000-0000-000000000000")
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn remove_user_other_tenant_no_op() {
    // arrange: tenant A invites; tenant B tries to delete.
    let app = TestApp::spawn().await;
    let id = invited_id(&app, "tenant_a_member@acme.test").await;
    let (_, other_token) = app.seed_other_user().await;

    // act
    let response = app
        .raw_client()
        .delete(format!("{}/admin/v1/users/{id}", app.addr()))
        .bearer_auth(other_token)
        .send()
        .await
        .unwrap();

    // assert: idempotent 204, but the row must survive (scoped by company_id).
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE user_id = $1")
            .bind(&id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn remove_user_malformed_id_validation_error() {
    let app = TestApp::spawn().await;
    let response = app.delete_user("not-a-uuid").await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn remove_user_missing_jwt_unauthorized() {
    let app = TestApp::spawn().await;
    let id = invited_id(&app, "needs_auth@acme.test").await;
    let response = app
        .raw_client()
        .delete(format!("{}/admin/v1/users/{id}", app.addr()))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn remove_user_as_member_forbidden() {
    // Only admins can remove members. A member-role caller gets 403; the
    // target row must survive.
    let app = TestApp::spawn().await;
    let target_id = invited_id(&app, "target@acme.test").await;
    let (_, member_token) = app.seed_member_in_same_company("member@acme.test").await;

    let response = app
        .raw_client()
        .delete(format!("{}/admin/v1/users/{target_id}", app.addr()))
        .bearer_auth(member_token)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE user_id = $1")
            .bind(&target_id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(count.0, 1, "target should survive a member's delete attempt");
}

#[tokio::test]
async fn removed_users_existing_jwt_is_rejected_on_next_call() {
    // Closes the "open tab keeps working" hole: a JWT only proves who you
    // *were* at mint time, so the middleware must re-check that the user
    // still exists. Without this guard, Bob's open browser window would keep
    // hitting admin endpoints until his JWT expired.

    // arrange: owner invites Bob, Bob "signs in" and claims the stub.
    let app = TestApp::spawn().await;
    let bob_id = invited_id(&app, "bob@acme.test").await;
    let db = ExperimentsDB::new(app.pool.clone());
    let bob_login = provision_and_mint(
        &db,
        &token_info("bob-google-sub", "bob@acme.test"),
        TEST_JWT_SECRET,
    )
    .await
    .expect("bob's first login claims the stub");

    // sanity: Bob's JWT works before removal.
    let before = app
        .raw_client()
        .get(format!("{}/admin/v1/users", app.addr()))
        .bearer_auth(&bob_login.token)
        .send()
        .await
        .unwrap();
    assert_eq!(before.status(), StatusCode::OK);

    // act: owner removes Bob.
    let removed = app.delete_user(&bob_id).await;
    assert_eq!(removed.status(), StatusCode::NO_CONTENT);

    // assert: Bob's same JWT is now refused.
    let after = app
        .raw_client()
        .get(format!("{}/admin/v1/users", app.addr()))
        .bearer_auth(&bob_login.token)
        .send()
        .await
        .unwrap();
    assert_eq!(after.status(), StatusCode::UNAUTHORIZED);
}
