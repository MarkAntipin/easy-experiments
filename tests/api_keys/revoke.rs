//! DELETE /admin/v1/api-keys/{id}

use reqwest::StatusCode;

use super::created_id;
use crate::common::TestApp;

#[tokio::test]
async fn revoke_api_key_existing_no_content() {
    // arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, "to_revoke").await;

    // act
    let response = app.delete_api_key(&id).await;

    // assert
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_keys WHERE api_key_id = $1")
        .bind(&id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0);
}

#[tokio::test]
async fn revoke_api_key_unknown_id_no_content() {
    // Revoke is idempotent — handler returns 204 even when nothing matched.
    let app = TestApp::spawn().await;

    // act
    let response = app
        .delete_api_key("00000000-0000-0000-0000-000000000000")
        .await;

    // assert
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn revoke_api_key_already_revoked_no_content() {
    // arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, "revoke_twice").await;
    assert_eq!(
        app.delete_api_key(&id).await.status(),
        StatusCode::NO_CONTENT
    );

    // act
    let response = app.delete_api_key(&id).await;

    // assert
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn revoke_api_key_malformed_id_validation_error() {
    // arrange
    let app = TestApp::spawn().await;

    // act
    let response = app.delete_api_key("not-a-uuid").await;

    // assert
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn revoke_api_key_other_tenant_no_op() {
    // arrange: tenant A creates a key, tenant B tries to revoke it.
    let app = TestApp::spawn().await;
    let id = created_id(&app, "tenant_a").await;
    let (_, other_token) = app.seed_other_user().await;

    // act: handler is idempotent so still 204, but the row must survive.
    let response = app
        .raw_client()
        .delete(format!("{}/admin/v1/api-keys/{id}", app.addr()))
        .bearer_auth(other_token)
        .send()
        .await
        .unwrap();

    // assert
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_keys WHERE api_key_id = $1")
        .bind(&id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn revoke_api_key_missing_jwt_unauthorized() {
    // arrange
    let app = TestApp::spawn().await;
    let id = created_id(&app, "needs_auth").await;

    // act: no bearer token
    let response = app
        .raw_client()
        .delete(format!("{}/admin/v1/api-keys/{id}", app.addr()))
        .send()
        .await
        .unwrap();

    // assert
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
