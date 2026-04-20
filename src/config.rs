use config::Environment;
use dotenv::dotenv;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default = "default_application_port")]
    pub application_port: u16,

    #[serde(default = "default_api_key")]
    pub api_key: String,

    #[serde(default = "default_sqlite_url")]
    pub sqlite_url: String,

    #[serde(default = "default_duckdb_path")]
    pub duckdb_path: String,
}

impl Config {
    pub fn sqlite_filepath(&self) -> PathBuf {
        let path = self.sqlite_url.strip_prefix("sqlite://")
            .expect("Invalid sqlite_url format");
        PathBuf::from(&path)
    }
}

fn default_application_port() -> u16 {
    18200
}

fn default_api_key() -> String {
    "api-key".to_string()
}

fn default_sqlite_url() -> String {
    "sqlite://easy-experiments.db".to_string()
}

fn default_duckdb_path() -> String {
    "easy-experiments.duckdb".to_string()
}

pub fn get_config() -> Result<Config, config::ConfigError> {
    dotenv().ok();

    let mut conf = config::Config::default();

    conf.merge(Environment::default().separator(""))?;

    conf.try_into()
}
