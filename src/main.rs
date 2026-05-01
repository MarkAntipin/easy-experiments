use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use env_logger::Env;
use easy_experiments::analytics::{spawn_writer, EventSink, ExposureEvent, MpscEventSink, WriterConfig};
use easy_experiments::config::get_config;
use easy_experiments::models::ExperimentsDB;
use easy_experiments::services::google_auth::GoogleTokenVerifier;
use easy_experiments::startup::run;
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqlitePool, SqliteConnectOptions, SqliteJournalMode};
use tokio::sync::mpsc;

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

    let experiments_db = ExperimentsDB::new(pool);

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

    let (event_tx, event_rx) = mpsc::channel::<ExposureEvent>(config.event_queue_capacity);
    let writer_handle = spawn_writer(
        event_rx,
        PathBuf::from(&config.duckdb_path),
        WriterConfig {
            batch_capacity: config.event_batch_capacity,
            flush_interval: Duration::from_millis(config.event_flush_interval_ms),
        },
    );
    let event_sink: Arc<dyn EventSink> = Arc::new(MpscEventSink::new(event_tx));

    run(
        listener,
        experiments_db,
        jwt_secret,
        google_verifier,
        cors_allowed_origins,
        event_sink,
    )?
    .await?;

    // Server has stopped accepting connections; the App and its sink Arc are
    // dropped, which closes the channel. Wait for the writer to drain.
    if let Err(e) = writer_handle.await {
        log::warn!("analytics writer task did not exit cleanly: {:?}", e);
    }

    Ok(())
}
