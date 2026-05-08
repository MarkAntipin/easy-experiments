use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use easy_experiments::config::get_config;
use easy_experiments::models::{ExperimentsDB, ExposureEvent, MetricEvent};
use easy_experiments::services::analytics::{DuckDBReadPool, ResultsService};
use easy_experiments::services::exposure::{
    bootstrap_duckdb_schema, spawn_sink_stats, spawn_writer, DedupConfig, EventSink, MpscEventSink,
    WriterConfig,
};
use easy_experiments::services::google_auth::GoogleTokenVerifier;
use easy_experiments::services::metric_sink::{
    spawn_metric_writer, MetricDedupConfig, MetricSink, MetricWriterConfig, MpscMetricSink,
};
use easy_experiments::startup::run;
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool};
use tokio::sync::mpsc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Initialize the tracing subscriber.
///
/// `RUST_LOG` controls levels (e.g. `info,sqlx=warn`). `LOG_FORMAT=json` emits
/// one JSON object per event for ingestion by journald/Docker/fly. Anything
/// else uses the human-readable formatter for local dev.
fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn,h2=warn,hyper=warn,reqwest=warn"));

    let json = std::env::var("LOG_FORMAT")
        .map(|v| v.eq_ignore_ascii_case("json"))
        .unwrap_or(false);

    if json {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().with_target(true).json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().with_target(true))
            .init();
    }
}

/// Augment the default panic hook with a tracing event so panics land in
/// structured logs alongside the default stderr backtrace.
fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(panic = %info, "thread panicked");
        default(info);
    }));
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_tracing();
    install_panic_hook();

    let config = get_config().expect("Failed to read configuration.");

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

    tracing::info!(
        port = config.application_port,
        sqlite = %config.sqlite_filepath().display(),
        duckdb = %config.duckdb_path,
        event_queue_capacity = config.event_queue_capacity,
        event_batch_capacity = config.event_batch_capacity,
        event_flush_interval_ms = config.event_flush_interval_ms,
        exposure_dedup_capacity = config.exposure_dedup_capacity,
        exposure_dedup_ttl_secs = config.exposure_dedup_ttl_secs,
        metric_queue_capacity = config.metric_queue_capacity,
        metric_batch_capacity = config.metric_batch_capacity,
        metric_flush_interval_ms = config.metric_flush_interval_ms,
        analytics_pool_size = config.analytics_pool_size,
        analytics_cache_ttl_secs = config.analytics_cache_ttl_secs,
        cors_allowed_origins = ?cors_allowed_origins,
        "starting easy-experiments",
    );

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

    let jwt_secret = config.jwt_secret.expect("JWT_SECRET must be set");
    let google_client_id = config.google_client_id.expect("GOOGLE_CLIENT_ID must be set");
    let google_verifier = GoogleTokenVerifier::new(google_client_id, config.google_jwks_url);

    let duckdb_path = PathBuf::from(&config.duckdb_path);
    bootstrap_duckdb_schema(&duckdb_path).expect("Failed to bootstrap DuckDB schema");

    let (event_tx, event_rx) = mpsc::channel::<ExposureEvent>(config.event_queue_capacity);
    let writer_handle = spawn_writer(
        event_rx,
        duckdb_path.clone(),
        WriterConfig {
            batch_capacity: config.event_batch_capacity,
            flush_interval: Duration::from_millis(config.event_flush_interval_ms),
        },
    );
    let event_sink: Arc<dyn EventSink> = Arc::new(MpscEventSink::with_dedup_config(
        event_tx,
        DedupConfig {
            max_capacity: config.exposure_dedup_capacity,
            ttl: Duration::from_secs(config.exposure_dedup_ttl_secs),
        },
    ));

    spawn_sink_stats(&event_sink, Duration::from_secs(30));

    let (metric_tx, metric_rx) = mpsc::channel::<MetricEvent>(config.metric_queue_capacity);
    let metric_writer_handle = spawn_metric_writer(
        metric_rx,
        duckdb_path.clone(),
        MetricWriterConfig {
            batch_capacity: config.metric_batch_capacity,
            flush_interval: Duration::from_millis(config.metric_flush_interval_ms),
        },
    );
    let metric_sink: Arc<dyn MetricSink> = Arc::new(MpscMetricSink::with_dedup_config(
        metric_tx,
        MetricDedupConfig {
            max_capacity: config.metric_idempotency_capacity,
            ttl: Duration::from_secs(config.metric_idempotency_ttl_secs),
        },
    ));

    let read_pool = Arc::new(DuckDBReadPool::new(
        duckdb_path,
        config.analytics_pool_size,
    ));
    let results_service = Arc::new(ResultsService::new(
        Arc::clone(&read_pool),
        config.analytics_cache_capacity,
        Duration::from_secs(config.analytics_cache_ttl_secs),
    ));

    run(
        listener,
        experiments_db,
        jwt_secret,
        google_verifier,
        cors_allowed_origins,
        event_sink,
        metric_sink,
        results_service,
    )?
    .await?;

    // Server has stopped accepting connections; the App and its sink Arcs are
    // dropped, which closes both channels. Wait for both writers to drain.
    if let Err(e) = writer_handle.await {
        tracing::warn!(error = %e, "exposure writer task did not exit cleanly");
    }
    if let Err(e) = metric_writer_handle.await {
        tracing::warn!(error = %e, "metric writer task did not exit cleanly");
    }

    Ok(())
}
