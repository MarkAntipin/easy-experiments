//! Handler-scoped test modules for `/admin/v1/users`.

mod invite;
mod list;
mod login_claim;
mod remove;

use reqwest::StatusCode;
use serde_json::{json, Value};

use crate::common::TestApp;

/// Invite a user via the admin route and return their userId. Panics on
/// non-201 — call sites treat this as an arrange step.
pub async fn invited_id(app: &TestApp, email: &str) -> String {
    let response = app.post_user(&json!({ "email": email })).await;
    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "invite_user should succeed as a test precondition"
    );
    response
        .json::<Value>()
        .await
        .unwrap()
        .get("userId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .expect("userId in response")
}
