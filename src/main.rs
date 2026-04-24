use std::net::TcpListener;
use env_logger::Env;
use easy_experiments::config::get_config;
use easy_experiments::models::ExperimentsDB;
use easy_experiments::services::google_auth::GoogleTokenVerifier;
use easy_experiments::startup::run;
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqlitePool, SqliteConnectOptions, SqliteJournalMode};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let config = get_config().expect("Failed to read configuration.");

    let db_options = SqliteConnectOptions::new()
        .filename(config.sqlite_filepath())
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePool::connect_with(db_options)
        .await
        .expect("Failed to create database pool");

    MIGRATOR.run(&pool).await.expect("Migration failed");

    let experiments_db = ExperimentsDB {
        pool
    };

    let address = format!("0.0.0.0:{}", config.application_port);
    let listener = TcpListener::bind(address)?;

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let jwt_secret = config.jwt_secret.expect("JWT_SECRET must be set");
    let google_client_id = config.google_client_id.expect("GOOGLE_CLIENT_ID must be set");
    let google_verifier = GoogleTokenVerifier::new(google_client_id, config.google_jwks_url);

    let cors_allowed_origins: Vec<String> = config
        .cors_allowed_origins
        .as_deref()
        .map(|raw| {
            raw.split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    run(
        listener,
        experiments_db,
        jwt_secret,
        google_verifier,
        cors_allowed_origins,
    )?
    .await
}
