use config::{Config as RawConfig, Environment};
use dotenv::dotenv;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub application_port: u16,

    pub api_key: Option<String>,

    pub jwt_secret: Option<String>,

    pub google_client_id: Option<String>,

    pub google_jwks_url: String,

    pub sqlite_url: String,

    pub duckdb_path: String,

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
}

pub fn get_config() -> Result<Config, config::ConfigError> {
    dotenv().ok();

    RawConfig::builder()
        .set_default("application_port", 18200)?
        .set_default("sqlite_url", "sqlite://easy-experiments.db")?
        .set_default("duckdb_path", "easy-experiments.duckdb")?
        .set_default(
            "google_jwks_url",
            crate::services::google_auth::DEFAULT_GOOGLE_JWKS_URL,
        )?
        .add_source(Environment::default().separator(""))
        .build()?
        .try_deserialize()
}
