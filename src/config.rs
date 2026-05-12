use config::{Config as RawConfig, Environment};
use dotenv::dotenv;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub application_port: u16,

    pub jwt_secret: Option<String>,

    pub google_client_id: Option<String>,

    pub google_jwks_url: String,

    /// Optional bootstrap admin. When set and the `users` table is empty on
    /// startup, a fresh company + admin user is seeded with this email/password
    /// so a brand-new self-hosted instance has someone who can sign in.
    pub admin_email: Option<String>,
    pub admin_password: Option<String>,
    pub admin_company_name: String,

    /// How long an accept-invite token stays valid. 14 days is the default —
    /// long enough to accommodate "I missed the link" without leaving stale
    /// tokens around for months.
    pub invite_token_ttl_days: u32,

    /// Public base URL where the UI lives. Used to render the `acceptInviteUrl`
    /// that the inviting admin copies to the new teammate. Defaults to empty,
    /// in which case only the bare `token` is returned and the UI builds the
    /// URL from its own origin.
    pub app_base_url: String,

    pub sqlite_url: String,

    pub duckdb_path: String,

    pub event_queue_capacity: usize,

    pub event_batch_capacity: usize,

    pub event_flush_interval_ms: u64,

    pub exposure_dedup_capacity: u64,

    pub exposure_dedup_ttl_secs: u64,

    pub metric_queue_capacity: usize,

    pub metric_batch_capacity: usize,

    pub metric_flush_interval_ms: u64,

    pub metric_idempotency_capacity: u64,

    pub metric_idempotency_ttl_secs: u64,

    pub analytics_pool_size: usize,

    pub analytics_cache_capacity: u64,

    pub analytics_cache_ttl_secs: u64,

    pub cors_allowed_origins: Option<String>,
}

impl Config {
    pub fn sqlite_filepath(&self) -> PathBuf {
        let path = self
            .sqlite_url
            .strip_prefix("sqlite://")
            .unwrap_or(&self.sqlite_url);
        PathBuf::from(path)
    }

    /// Pick the auth provider based on what's configured:
    /// `GOOGLE_CLIENT_ID` set → Google sign-in only (hosted SaaS shape).
    /// Otherwise → email + password (OSS / self-hosted shape).
    ///
    /// Mutually exclusive on purpose: one signal, one mode, no env var to
    /// remember. Operators wanting to test both flows side-by-side in dev can
    /// flip the signal by un/setting `GOOGLE_CLIENT_ID`.
    pub fn auth_provider_set(&self) -> AuthProviders {
        let google_configured = self
            .google_client_id
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if google_configured {
            AuthProviders {
                google: true,
                password: false,
            }
        } else {
            AuthProviders {
                google: false,
                password: true,
            }
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct AuthProviders {
    pub google: bool,
    pub password: bool,
}

pub fn get_config() -> Result<Config, config::ConfigError> {
    dotenv().ok();

    RawConfig::builder()
        .set_default("application_port", 18200)?
        .set_default("admin_company_name", "Default")?
        .set_default("invite_token_ttl_days", 14)?
        .set_default("app_base_url", "")?
        .set_default("sqlite_url", "sqlite://easy-experiments.db")?
        .set_default("duckdb_path", "easy-experiments.duckdb")?
        .set_default("event_queue_capacity", 10_000)?
        .set_default("event_batch_capacity", 1_000)?
        .set_default("event_flush_interval_ms", 1_000)?
        .set_default("exposure_dedup_capacity", 1_000_000)?
        .set_default("exposure_dedup_ttl_secs", 3_600)?
        .set_default("metric_queue_capacity", 10_000)?
        .set_default("metric_batch_capacity", 1_000)?
        .set_default("metric_flush_interval_ms", 1_000)?
        .set_default("metric_idempotency_capacity", 200_000)?
        .set_default("metric_idempotency_ttl_secs", 600)?
        .set_default("analytics_pool_size", 4)?
        .set_default("analytics_cache_capacity", 1_000)?
        .set_default("analytics_cache_ttl_secs", 30)?
        .set_default(
            "google_jwks_url",
            crate::services::google_auth::DEFAULT_GOOGLE_JWKS_URL,
        )?
        .add_source(Environment::default().separator(""))
        .build()?
        .try_deserialize()
}
