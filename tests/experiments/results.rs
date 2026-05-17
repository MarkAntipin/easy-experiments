//! GET /admin/v1/experiments/{id}/results

use chrono::Utc;
use reqwest::StatusCode;
use serde_json::Value;
use uuid::Uuid;

use crate::common::{valid_experiment_body, TestApp};

#[tokio::test]
async fn get_results_unknown_id_not_found() {
    // arrange
    let app = TestApp::spawn().await;

    // act
    let response = app
        .get_experiment_results("00000000-0000-0000-0000-000000000000")
        .await;

    // assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_results_other_tenant_not_found() {
    // arrange: tenant A creates an experiment, tenant B asks for its results.
    let app = TestApp::spawn().await;
    let id = app
        .running_experiment(&valid_experiment_body("scoping"))
        .await;
    let (_, other_token) = app.seed_other_user().await;

    // act
    let response = app
        .raw_client()
        .get(format!("{}/admin/v1/experiments/{id}/results", app.addr()))
        .bearer_auth(other_token)
        .send()
        .await
        .unwrap();

    // assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_results_aggregates_exposures_and_conversions() {
    // arrange: a running experiment with a pinned start time so seeded
    // events land predictably inside the [start_ms, end_ms] window.
    let app = TestApp::spawn().await;
    let id = app.running_experiment(&valid_experiment_body("agg")).await;

    let started_at = Utc::now().timestamp_millis() - 2 * 60 * 60 * 1000;
    app.set_experiment_started_at(&id, started_at).await;

    let event_ts = started_at + 60 * 1000;
    let company_id = app.user.company_id.clone();

    // 3 control exposures, 1 converter; 2 treatment exposures, 1 converter.
    // `primaryMetric` on the fixture is "conversion_rate", so the seeded
    // metric_name must match for attribution.
    for entity in ["u1", "u2", "u3"] {
        app.seed_exposure(&company_id, &id, "control", entity, event_ts);
    }
    app.seed_metric_event(&company_id, "u1", "conversion_rate", event_ts + 1);

    for entity in ["u4", "u5"] {
        app.seed_exposure(&company_id, &id, "treatment", entity, event_ts);
    }
    app.seed_metric_event(&company_id, "u5", "conversion_rate", event_ts + 1);

    // act
    let response = app.get_experiment_results(&id).await;

    // assert
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["experimentId"], id);
    assert_eq!(body["experimentKey"], "agg");

    let variants = body["variants"].as_array().expect("variants array");
    let control = variants
        .iter()
        .find(|v| v["variantKey"] == "control")
        .expect("control row in response");
    let treatment = variants
        .iter()
        .find(|v| v["variantKey"] == "treatment")
        .expect("treatment row in response");

    assert_eq!(control["exposures"], 3);
    assert_eq!(control["converters"], 1);
    assert_eq!(treatment["exposures"], 2);
    assert_eq!(treatment["converters"], 1);
}

#[tokio::test]
async fn get_results_scopes_analytics_queries_to_tenant() {
    // arrange: seed exposures for this experiment_id under a *different*
    // company_id. If the analytics SQL ever drops its `company_id = ?`
    // filter, these rows would leak into the tenant's results and this
    // test would catch it.
    let app = TestApp::spawn().await;
    let id = app
        .running_experiment(&valid_experiment_body("scope_check"))
        .await;

    let started_at = Utc::now().timestamp_millis() - 2 * 60 * 60 * 1000;
    app.set_experiment_started_at(&id, started_at).await;

    let event_ts = started_at + 60 * 1000;
    let foreign_company = format!("other-{}", Uuid::new_v4());
    for entity in ["e1", "e2", "e3"] {
        app.seed_exposure(&foreign_company, &id, "control", entity, event_ts);
    }

    // act
    let response = app.get_experiment_results(&id).await;

    // assert
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    let variants = body["variants"].as_array().expect("variants array");
    assert!(!variants.is_empty(), "variants should still be reported");
    for v in variants {
        assert_eq!(
            v["exposures"], 0,
            "variant {} should not see other-tenant exposures",
            v["variantKey"]
        );
    }
}
