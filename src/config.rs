use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug)]
pub struct ConfigError(String);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "config error: {}", self.0)
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug)]
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

    /// How long an accept-invite token stays valid. 7 days is the default.
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

    /// Absolute path to a built UI bundle (Vite `dist/`). When set and the
    /// directory exists, the backend serves it on `/` with an SPA fallback,
    /// so a single Docker image ships both the API and admin UI. Unset in
    /// dev/test — `cargo run` then serves API only.
    pub ui_dist_path: Option<String>,
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
    /// `GOOGLE_CLIENT_ID` set → Google sign-in only.
    /// `ADMIN_EMAIL` and `ADMIN_PASSWORD` → email + password sign-in only.
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

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_parsed<T: FromStr>(key: &str, default: T) -> Result<T, ConfigError>
where
    T::Err: std::fmt::Display,
{
    match std::env::var(key) {
        Ok(v) => v
            .parse::<T>()
            .map_err(|e| ConfigError(format!("{key}: {e}"))),
        Err(_) => Ok(default),
    }
}

pub fn get_config() -> Result<Config, ConfigError> {
    dotenvy::dotenv().ok();

    Ok(Config {
        application_port: env_parsed("APPLICATION_PORT", 18200)?,
        jwt_secret: env_opt("JWT_SECRET"),
        google_client_id: env_opt("GOOGLE_CLIENT_ID"),
        google_jwks_url: env_or(
            "GOOGLE_JWKS_URL",
            crate::services::google_auth::DEFAULT_GOOGLE_JWKS_URL,
        ),
        admin_email: env_opt("ADMIN_EMAIL"),
        admin_password: env_opt("ADMIN_PASSWORD"),
        admin_company_name: env_or("ADMIN_COMPANY_NAME", "Default"),
        invite_token_ttl_days: env_parsed("INVITE_TOKEN_TTL_DAYS", 7)?,
        app_base_url: env_or("APP_BASE_URL", ""),
        sqlite_url: env_or("SQLITE_URL", "sqlite://easy-experiments.db"),
        duckdb_path: env_or("DUCKDB_PATH", "easy-experiments.duckdb"),
        event_queue_capacity: env_parsed("EVENT_QUEUE_CAPACITY", 10_000)?,
        event_batch_capacity: env_parsed("EVENT_BATCH_CAPACITY", 1_000)?,
        event_flush_interval_ms: env_parsed("EVENT_FLUSH_INTERVAL_MS", 1_000)?,
        exposure_dedup_capacity: env_parsed("EXPOSURE_DEDUP_CAPACITY", 1_000_000)?,
        exposure_dedup_ttl_secs: env_parsed("EXPOSURE_DEDUP_TTL_SECS", 3_600)?,
        metric_queue_capacity: env_parsed("METRIC_QUEUE_CAPACITY", 10_000)?,
        metric_batch_capacity: env_parsed("METRIC_BATCH_CAPACITY", 1_000)?,
        metric_flush_interval_ms: env_parsed("METRIC_FLUSH_INTERVAL_MS", 1_000)?,
        metric_idempotency_capacity: env_parsed("METRIC_IDEMPOTENCY_CAPACITY", 200_000)?,
        metric_idempotency_ttl_secs: env_parsed("METRIC_IDEMPOTENCY_TTL_SECS", 600)?,
        analytics_pool_size: env_parsed("ANALYTICS_POOL_SIZE", 4)?,
        analytics_cache_capacity: env_parsed("ANALYTICS_CACHE_CAPACITY", 1_000)?,
        analytics_cache_ttl_secs: env_parsed("ANALYTICS_CACHE_TTL_SECS", 30)?,
        cors_allowed_origins: env_opt("CORS_ALLOWED_ORIGINS"),
        ui_dist_path: env_opt("UI_DIST_PATH"),
    })
}
