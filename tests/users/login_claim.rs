//! Tests the auth-side half of the invite flow: when an invited (pending) user
//! signs in with Google, the existing stub row is claimed (its google_sub gets
//! populated) rather than a new tenant being auto-created.
//!
//! These bypass the HTTP layer because the test harness can't mint a real
//! Google-signed token; we drive `services::auth::provision_and_mint` directly
//! with a constructed `GoogleTokenInfo`.

use easy_experiments::models::ExperimentsDB;
use easy_experiments::services::auth::provision_and_mint;
use easy_experiments::services::google_auth::GoogleTokenInfo;
use reqwest::StatusCode;
use serde_json::json;

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
async fn pending_user_is_claimed_on_first_login() {
    // arrange: invite alice; she has no google_sub yet.
    let app = TestApp::spawn().await;
    let invited_user_id = invited_id(&app, "alice@acme.test").await;
    let db = ExperimentsDB::new(app.pool.clone());

    // act: alice signs in with Google. token email matches her invite.
    let result = provision_and_mint(
        &db,
        &token_info("google-sub-for-alice", "alice@acme.test"),
        TEST_JWT_SECRET,
    )
    .await
    .expect("provision should succeed");

    // assert: claimed the existing stub row (same user_id), landed in acme,
    // and the sub is now bound on the DB row.
    assert_eq!(result.user.user_id, invited_user_id);
    assert_eq!(result.user.company_id, app.user.company_id);
    assert_eq!(
        result.user.google_sub.as_deref(),
        Some("google-sub-for-alice")
    );
    assert!(!result.token.is_empty());

    let row: (String, String) =
        sqlx::query_as("SELECT company_id, google_sub FROM users WHERE user_id = $1")
            .bind(&invited_user_id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row.0, app.user.company_id);
    assert_eq!(row.1, "google-sub-for-alice");
}

#[tokio::test]
async fn claimed_user_can_use_their_jwt() {
    // arrange: invite, then "log in" to claim.
    let app = TestApp::spawn().await;
    invited_id(&app, "alice@acme.test").await;
    let db = ExperimentsDB::new(app.pool.clone());
    let result = provision_and_mint(
        &db,
        &token_info("google-sub-for-alice", "alice@acme.test"),
        TEST_JWT_SECRET,
    )
    .await
    .expect("provision should succeed");

    // act: hit an admin endpoint with alice's freshly-minted JWT.
    let response = app
        .raw_client()
        .get(format!("{}/admin/v1/users", app.addr()))
        .bearer_auth(&result.token)
        .send()
        .await
        .unwrap();

    // assert: alice is authenticated as a member of the acme tenant.
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
}

#[tokio::test]
async fn email_normalized_when_claiming_stub() {
    // arrange: stub stored as lowercase, token arrives mixed-case.
    let app = TestApp::spawn().await;
    let invited_user_id = invited_id(&app, "alice@acme.test").await;
    let db = ExperimentsDB::new(app.pool.clone());

    // act
    let result = provision_and_mint(
        &db,
        &token_info("google-sub-for-alice", "  Alice@ACME.test  "),
        TEST_JWT_SECRET,
    )
    .await
    .expect("provision should succeed (email is normalized before lookup)");

    // assert
    assert_eq!(result.user.user_id, invited_user_id);
    assert_eq!(result.user.company_id, app.user.company_id);
}

#[tokio::test]
async fn unknown_email_still_auto_creates_company() {
    // No invite, no existing user → falls through to self-serve signup
    // (current behavior). We assert this explicitly so the regression is
    // caught if anyone tightens it later.
    let app = TestApp::spawn().await;
    let db = ExperimentsDB::new(app.pool.clone());

    // act
    let result = provision_and_mint(
        &db,
        &token_info("brand-new-sub", "stranger@somewhere.test"),
        TEST_JWT_SECRET,
    )
    .await
    .expect("self-serve signup should succeed");

    // assert: new company, not the seeded acme.
    assert_ne!(result.user.company_id, app.user.company_id);
    assert_eq!(result.user.email, "stranger@somewhere.test");
}

#[tokio::test]
async fn email_collision_with_active_user_is_rejected() {
    // arrange: alice exists in the acme company with a real google_sub
    // (we simulate this by inviting then claiming with sub-A).
    let app = TestApp::spawn().await;
    invited_id(&app, "alice@acme.test").await;
    let db = ExperimentsDB::new(app.pool.clone());
    provision_and_mint(
        &db,
        &token_info("sub-A", "alice@acme.test"),
        TEST_JWT_SECRET,
    )
    .await
    .expect("first login claims the stub");

    // act: a *different* Google account with the same email attempts to sign
    // in. The existing identity binding must be preserved.
    let result = provision_and_mint(
        &db,
        &token_info("sub-B-different", "alice@acme.test"),
        TEST_JWT_SECRET,
    )
    .await;
    assert!(result.is_err(), "colliding sub must be rejected");
    let err = result.err().unwrap();

    // assert: surfaces as a 409 to the caller.
    use actix_web::ResponseError;
    assert_eq!(err.status_code().as_u16(), 409);
}

#[tokio::test]
async fn cannot_post_to_admin_users_when_email_collides_across_tenants() {
    // Composite check: tenant A invites bob. Bob — unaware — does a
    // first-time Google sign-in himself with the same email. He should land
    // in tenant A (claim the stub) rather than auto-creating a separate
    // company (which would 500 on the email UNIQUE constraint).
    let app = TestApp::spawn().await;
    let invited_user_id = invited_id(&app, "bob@acme.test").await;
    let db = ExperimentsDB::new(app.pool.clone());

    let result = provision_and_mint(
        &db,
        &token_info("bob-google-sub", "bob@acme.test"),
        TEST_JWT_SECRET,
    )
    .await
    .expect("bob's first sign-in claims the stub");

    assert_eq!(result.user.user_id, invited_user_id);
    assert_eq!(result.user.company_id, app.user.company_id);

    let listing = app
        .raw_client()
        .get(format!("{}/admin/v1/users", app.addr()))
        .bearer_auth(&result.token)
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = listing.json().await.unwrap();
    let items = body["items"].as_array().unwrap();
    let bob = items
        .iter()
        .find(|i| i["email"] == json!("bob@acme.test"))
        .expect("bob in member list");
    assert_eq!(bob["status"], "active");
}
