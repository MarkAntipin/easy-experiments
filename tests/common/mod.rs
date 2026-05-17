//! Shared test harness for integration tests.
//!
//! Each test gets:
//!   * A fresh in-memory SQLite database (unique URI + shared cache, so the DB
//!     is dropped automatically when the pool goes out of scope - no files on
//!     disk, no cleanup required).
//!   * Migrations applied.
//!   * A seeded company + user.
//!   * A signed JWT for that user.
//!   * A running actix-web server on a random port, hit via `reqwest`.

use std::net::TcpListener;
use std::str::FromStr;
use std::sync::Arc;

use chrono::Utc;
use reqwest::Client;
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use uuid::Uuid;

use easy_experiments::config::AuthProviders;
use easy_experiments::models::{AuthenticatedUser, ExperimentsDB, InviteConfig, UserRole};
use easy_experiments::repository::duckdb::open_and_bootstrap;
use easy_experiments::services::analytics::{DuckDBReadPool, ResultsService};
use easy_experiments::services::exposure::{EventSink, NoopEventSink};
use easy_experiments::services::google_auth::{GoogleTokenVerifier, DEFAULT_GOOGLE_JWKS_URL};
use easy_experiments::services::jwt::create_jwt;
use easy_experiments::services::metric_sink::{MetricSink, NoopMetricSink};
use easy_experiments::startup::run;

const TEST_JWT_SECRET: &str = "integration-test-jwt-secret";

pub struct TestApp {
    pub address: String,
    pub pool: SqlitePool,
    pub user: AuthenticatedUser,
    pub token: String,
    client: Client,
    duckdb_writer: std::sync::Mutex<duckdb::Connection>,
}

// Each `tests/*.rs` compiles `common` separately; helpers used by one binary
// look "dead" to another. Suppress at the impl level so we don't have to
// annotate every method.
#[allow(dead_code)]
impl TestApp {
    pub async fn spawn() -> Self {
        spawn_app().await
    }

    /// Seed a second company+user and return a JWT for them. Useful for
    /// testing multi-tenant isolation. The seeded user is an admin (matches
    /// the real-world rule that the company creator owns the workspace).
    pub async fn seed_other_user(&self) -> (AuthenticatedUser, String) {
        let (user, _) =
            seed_company_and_user(&self.pool, "other-co", "other@example.com", UserRole::Admin)
                .await;
        let token = create_jwt(&user, TEST_JWT_SECRET).expect("mint jwt");
        (user, token)
    }

    /// Seed an additional `member`-role user inside the *current* tenant and
    /// return a JWT for them. Used to test that non-admins can't invite or
    /// remove.
    pub async fn seed_member_in_same_company(&self, email: &str) -> (AuthenticatedUser, String) {
        let user =
            seed_user_in_company(&self.pool, &self.user.company_id, email, UserRole::Member).await;
        let token = create_jwt(&user, TEST_JWT_SECRET).expect("mint jwt");
        (user, token)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.address, path)
    }

    pub async fn post_experiment(&self, body: &Value) -> reqwest::Response {
        self.client
            .post(self.url("/admin/v1/experiments"))
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await
            .expect("POST /admin/v1/experiments")
    }

    pub async fn get_experiment(&self, id: &str) -> reqwest::Response {
        self.client
            .get(self.url(&format!("/admin/v1/experiments/{id}")))
            .bearer_auth(&self.token)
            .send()
            .await
            .expect("GET /admin/v1/experiments/{id}")
    }

    pub async fn list_experiments(&self, status: Option<&str>) -> reqwest::Response {
        let mut req = self
            .client
            .get(self.url("/admin/v1/experiments"))
            .bearer_auth(&self.token);
        if let Some(s) = status {
            req = req.query(&[("status", s)]);
        }
        req.send().await.expect("GET /admin/v1/experiments")
    }

    pub async fn patch_experiment(
        &self,
        id: &str,
        body: &Value,
        if_match: Option<i64>,
    ) -> reqwest::Response {
        let mut req = self
            .client
            .patch(self.url(&format!("/admin/v1/experiments/{id}")))
            .bearer_auth(&self.token)
            .json(body);
        if let Some(v) = if_match {
            req = req.header("If-Match", v.to_string());
        }
        req.send().await.expect("PATCH /admin/v1/experiments/{id}")
    }

    pub async fn delete_experiment(&self, id: &str) -> reqwest::Response {
        self.client
            .delete(self.url(&format!("/admin/v1/experiments/{id}")))
            .bearer_auth(&self.token)
            .send()
            .await
            .expect("DELETE /admin/v1/experiments/{id}")
    }

    pub async fn start_experiment(&self, id: &str) -> reqwest::Response {
        self.client
            .post(self.url(&format!("/admin/v1/experiments/{id}/start")))
            .bearer_auth(&self.token)
            .send()
            .await
            .expect("POST /admin/v1/experiments/{id}/start")
    }

    pub async fn stop_experiment(&self, id: &str) -> reqwest::Response {
        self.client
            .post(self.url(&format!("/admin/v1/experiments/{id}/stop")))
            .bearer_auth(&self.token)
            .send()
            .await
            .expect("POST /admin/v1/experiments/{id}/stop")
    }

    pub async fn get_experiment_results(&self, id: &str) -> reqwest::Response {
        self.client
            .get(self.url(&format!("/admin/v1/experiments/{id}/results")))
            .bearer_auth(&self.token)
            .send()
            .await
            .expect("GET /admin/v1/experiments/{id}/results")
    }

    pub fn seed_exposure(
        &self,
        company_id: &str,
        experiment_id: &str,
        variant_key: &str,
        entity_id: &str,
        ts_ms: i64,
    ) {
        let conn = self.duckdb_writer.lock().expect("duckdb writer poisoned");
        conn.execute(
            "INSERT INTO exposures (schema_version, ts_ms, company_id, experiment_id, variant_key, entity_id) \
             VALUES (1, ?, ?, ?, ?, ?)",
            duckdb::params![ts_ms, company_id, experiment_id, variant_key, entity_id],
        )
        .expect("insert exposure");
    }

    /// Insert a row into DuckDB `metric_events` directly. See `seed_exposure`.
    pub fn seed_metric_event(
        &self,
        company_id: &str,
        entity_id: &str,
        metric_name: &str,
        ts_ms: i64,
    ) {
        let conn = self.duckdb_writer.lock().expect("duckdb writer poisoned");
        conn.execute(
            "INSERT INTO metric_events (schema_version, ts_ms, company_id, entity_id, metric_name, metric_value) \
             VALUES (1, ?, ?, ?, ?, NULL)",
            duckdb::params![ts_ms, company_id, entity_id, metric_name],
        )
        .expect("insert metric event");
    }

    pub async fn set_experiment_started_at(&self, id: &str, started_at_ms: i64) {
        sqlx::query("UPDATE experiments SET started_at = $1 WHERE experiment_id = $2")
            .bind(started_at_ms)
            .bind(id)
            .execute(&self.pool)
            .await
            .expect("update started_at");
    }

    pub async fn post_api_key(&self, body: &Value) -> reqwest::Response {
        self.client
            .post(self.url("/admin/v1/api-keys"))
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await
            .expect("POST /admin/v1/api-keys")
    }

    pub async fn list_api_keys(&self) -> reqwest::Response {
        self.client
            .get(self.url("/admin/v1/api-keys"))
            .bearer_auth(&self.token)
            .send()
            .await
            .expect("GET /admin/v1/api-keys")
    }

    pub async fn delete_api_key(&self, id: &str) -> reqwest::Response {
        self.client
            .delete(self.url(&format!("/admin/v1/api-keys/{id}")))
            .bearer_auth(&self.token)
            .send()
            .await
            .expect("DELETE /admin/v1/api-keys/{id}")
    }

    pub async fn post_user(&self, body: &Value) -> reqwest::Response {
        self.client
            .post(self.url("/admin/v1/users"))
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await
            .expect("POST /admin/v1/users")
    }

    pub async fn list_users(&self) -> reqwest::Response {
        self.client
            .get(self.url("/admin/v1/users"))
            .bearer_auth(&self.token)
            .send()
            .await
            .expect("GET /admin/v1/users")
    }

    pub async fn delete_user(&self, id: &str) -> reqwest::Response {
        self.client
            .delete(self.url(&format!("/admin/v1/users/{id}")))
            .bearer_auth(&self.token)
            .send()
            .await
            .expect("DELETE /admin/v1/users/{id}")
    }

    /// Provision an API key for the test user via the service layer (bypasses
    /// the admin route to keep the evaluate suite focused on /evaluate).
    /// Returns the plaintext to put in `X-Api-Key`.
    pub async fn seed_api_key(&self) -> String {
        self.seed_api_key_for(&self.user.company_id).await
    }

    pub async fn seed_api_key_for(&self, company_id: &str) -> String {
        let db = ExperimentsDB::new(self.pool.clone());
        easy_experiments::services::api_key::create(
            &db,
            company_id,
            "integration-test-key".to_string(),
        )
        .await
        .expect("create api key")
        .plaintext
    }

    pub async fn evaluate(&self, api_key: &str, body: &Value) -> reqwest::Response {
        self.client
            .post(self.url("/api/v1/experiments/evaluate"))
            .header("X-Api-Key", api_key)
            .json(body)
            .send()
            .await
            .expect("POST /api/v1/experiments/evaluate")
    }

    /// Create + start an experiment in one call. Returns its experiment_id.
    pub async fn running_experiment(&self, body: &Value) -> String {
        let create = self.post_experiment(body).await;
        assert!(
            create.status().is_success(),
            "create_experiment precondition failed: {}",
            create.status()
        );
        let id = create.json::<Value>().await.unwrap()["experimentId"]
            .as_str()
            .expect("experimentId")
            .to_string();
        let start = self.start_experiment(&id).await;
        assert!(
            start.status().is_success(),
            "start_experiment precondition failed: {}",
            start.status()
        );
        id
    }

    /// A bare reqwest client for crafting requests that need unusual auth etc.
    pub fn raw_client(&self) -> &Client {
        &self.client
    }

    pub fn addr(&self) -> &str {
        &self.address
    }

    /// Seed a user inside the current company with a real argon2 password hash.
    /// Used by the password-auth tests to skip the invite-accept dance when
    /// they just want to exercise login.
    pub async fn seed_password_user(&self, email: &str, password: &str, role: UserRole) -> String {
        let user_id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp_millis();
        let hash =
            easy_experiments::services::password::hash_password(password).expect("hash password");

        sqlx::query(
            "INSERT INTO users (
                user_id, company_id, email, name, picture_url,
                google_sub, password_hash, invite_token_hash, invite_token_expires_at,
                role, created_at, updated_at
            )
            VALUES ($1, $2, $3, NULL, NULL, NULL, $4, NULL, NULL, $5, $6, $6)",
        )
        .bind(&user_id)
        .bind(&self.user.company_id)
        .bind(email)
        .bind(&hash)
        .bind(role)
        .bind(now)
        .execute(&self.pool)
        .await
        .expect("insert password user");

        user_id
    }

    pub async fn password_login(&self, email: &str, password: &str) -> reqwest::Response {
        self.client
            .post(self.url("/admin/v1/auth/login"))
            .json(&serde_json::json!({ "email": email, "password": password }))
            .send()
            .await
            .expect("POST /admin/v1/auth/login")
    }

    pub async fn accept_invite(&self, token: &str, password: &str) -> reqwest::Response {
        self.client
            .post(self.url("/admin/v1/auth/accept-invite"))
            .json(&serde_json::json!({ "token": token, "password": password }))
            .send()
            .await
            .expect("POST /admin/v1/auth/accept-invite")
    }
}

async fn spawn_app() -> TestApp {
    let pool = build_pool().await;
    run_migrations(&pool).await;
    let (user, _) = seed_company_and_user(&pool, "acme", "owner@acme.test", UserRole::Admin).await;
    let token = create_jwt(&user, TEST_JWT_SECRET).expect("mint jwt");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{port}");

    let verifier = GoogleTokenVerifier::new(
        "test-google-client-id".to_string(),
        DEFAULT_GOOGLE_JWKS_URL.to_string(),
    );

    let db = ExperimentsDB::new(pool.clone());
    let event_sink: Arc<dyn EventSink> = Arc::new(NoopEventSink);
    let metric_sink: Arc<dyn MetricSink> = Arc::new(NoopMetricSink);

    // Per-test DuckDB file so the read pool has real (empty) tables to query.
    // Path is unique per test; the file is dropped when the temp dir is.
    let duckdb_dir = std::env::temp_dir().join(format!("ee-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&duckdb_dir).expect("create duckdb dir");
    let duckdb_path = duckdb_dir.join("test.duckdb");
    let duckdb_root = open_and_bootstrap(&duckdb_path).expect("bootstrap duckdb schema");
    let duckdb_writer = std::sync::Mutex::new(
        duckdb_root.try_clone().expect("clone duckdb writer session"),
    );
    let read_pool = Arc::new(DuckDBReadPool::new(duckdb_root, 2));
    let results_service = Arc::new(ResultsService::new(
        read_pool,
        16,
        std::time::Duration::from_secs(30),
    ));

    let providers = AuthProviders {
        google: true,
        password: true,
    };
    let invite_config = InviteConfig {
        token_ttl_days: 14,
        app_base_url: "http://localhost".to_string(),
    };

    let server = run(
        listener,
        db,
        TEST_JWT_SECRET.to_string(),
        Some(verifier),
        providers,
        invite_config,
        Vec::new(),
        event_sink,
        metric_sink,
        results_service,
        None,
        Default::default(),
    )
    .expect("start server");

    // Runs until the test's tokio runtime is dropped.
    tokio::spawn(server);

    TestApp {
        address,
        pool,
        user,
        token,
        client: Client::new(),
        duckdb_writer,
    }
}

async fn build_pool() -> SqlitePool {
    // Unique shared-cache in-memory DB per test. Multiple connections can
    // attach to the same DB via the URI name; the DB is freed when the last
    // connection closes (i.e. when this pool is dropped).
    let uri = format!("file:memdb_{}?mode=memory&cache=shared", Uuid::new_v4());
    let options = SqliteConnectOptions::from_str(&uri)
        .expect("parse sqlite uri")
        .foreign_keys(true);
    SqlitePool::connect_with(options)
        .await
        .expect("open in-memory sqlite")
}

async fn run_migrations(pool: &SqlitePool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("apply migrations");
}

async fn seed_company_and_user(
    pool: &SqlitePool,
    company_name: &str,
    email: &str,
    role: UserRole,
) -> (AuthenticatedUser, String) {
    let now = Utc::now().timestamp_millis();
    let company_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO companies (company_id, name, created_at, updated_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(&company_id)
    .bind(company_name)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert company");

    let user = seed_user_in_company(pool, &company_id, email, role).await;
    (user, company_id)
}

#[allow(dead_code)]
async fn seed_user_in_company(
    pool: &SqlitePool,
    company_id: &str,
    email: &str,
    role: UserRole,
) -> AuthenticatedUser {
    let now = Utc::now().timestamp_millis();
    let user_id = Uuid::new_v4().to_string();
    let google_sub = format!("test-sub-{}", Uuid::new_v4());

    sqlx::query(
        "INSERT INTO users (user_id, company_id, email, name, picture_url, google_sub, role, created_at, updated_at)
         VALUES ($1, $2, $3, NULL, NULL, $4, $5, $6, $6)",
    )
    .bind(&user_id)
    .bind(company_id)
    .bind(email)
    .bind(&google_sub)
    .bind(role)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert user");

    AuthenticatedUser {
        user_id,
        company_id: company_id.to_string(),
        email: email.to_string(),
        role,
    }
}

// -- Fixture builders ---------------------------------------------------------

/// A valid, minimal experiment body. Tweak fields on the returned `Value` to
/// construct edge cases.
#[allow(dead_code)] // Not used by every `tests/*.rs` binary that compiles `common`.
pub fn valid_experiment_body(key: &str) -> Value {
    serde_json::json!({
        "key": key,
        "description": "A test experiment",
        "primaryMetric": "conversion_rate",
        "variants": [
            { "key": "control", "isControl": true, "config": {} },
            { "key": "treatment", "isControl": false, "config": {} }
        ],
        "segments": [
            {
                "priority": 0,
                "rolloutPercent": 100,
                "constraints": [],
                "distributions": [
                    { "variantKey": "control",   "percent": 50 },
                    { "variantKey": "treatment", "percent": 50 }
                ]
            }
        ]
    })
}
