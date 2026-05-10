//! Handler-scoped test modules for `/admin/v1/api-keys`.

mod create;
mod list;
mod revoke;

// Shared helpers for this suite (used by handler submodules).

use reqwest::StatusCode;
use serde_json::{json, Value};

use crate::common::TestApp;

/// Create an API key via the admin route and return its id. Panics if the
/// POST does not return 201 — call sites treat this as an arrange-step
/// precondition.
pub async fn created_id(app: &TestApp, name: &str) -> String {
    let response = app.post_api_key(&json!({ "name": name })).await;
    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "create_api_key should succeed as a test precondition"
    );
    response
        .json::<Value>()
        .await
        .unwrap()
        .get("apiKeyId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .expect("apiKeyId in response")
}
