//! Handler-scoped test modules for `/admin/v1/experiments`.

mod create;
mod delete;
mod get;
mod list;
mod results;
mod start_stop;
mod update;

// Shared helpers for this suite (used by handler submodules).

use reqwest::StatusCode;
use serde_json::Value;

use crate::common::TestApp;

/// Create an experiment via the API and return its id. Panics if the POST
/// does not return 201 — call sites treat this as an arrange-step precondition.
pub async fn created_id(app: &TestApp, body: &Value) -> String {
    let response = app.post_experiment(body).await;
    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "create_experiment should succeed as a test precondition"
    );
    response
        .json::<Value>()
        .await
        .unwrap()
        .get("experimentId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .expect("experimentId in response")
}
