use chrono::Utc;
use easy_experiments::config::get_config;
use easy_experiments::models::{
    Constraint, ConstraintOperator, CreateExperimentRequest, Distribution, ExperimentsDB, Segment,
    Variant,
};
use easy_experiments::services::{api_key, experiment};
use serde_json::json;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool};

const COMPANY_ID: &str = "dc15994d-65fc-4ecd-90a5-7446cfedd7ed";

fn experiments() -> Vec<CreateExperimentRequest> {
    let control = || Variant {
        key: "control".into(),
        is_control: true,
        config: json!({"enabled": false}),
    };
    let treatment = |cfg: serde_json::Value| Variant {
        key: "treatment".into(),
        is_control: false,
        config: cfg,
    };
    let split_50_50 = || {
        vec![
            Distribution { variant_key: "control".into(), percent: 50 },
            Distribution { variant_key: "treatment".into(), percent: 50 },
        ]
    };

    vec![
        CreateExperimentRequest {
            key: "lt-checkout-flow".into(),
            description: Some("Loadtest: checkout flow".into()),
            primary_metric: "conversion_rate".into(),
            variants: vec![control(), treatment(json!({"enabled": true}))],
            segments: vec![Segment {
                priority: 0,
                rollout_percent: 100,
                constraints: vec![],
                distributions: split_50_50(),
            }],
        },
        CreateExperimentRequest {
            key: "lt-pricing-test".into(),
            description: Some("Loadtest: pricing test".into()),
            primary_metric: "conversion_rate".into(),
            variants: vec![control(), treatment(json!({"discount": 0.1}))],
            segments: vec![Segment {
                priority: 0,
                rollout_percent: 50,
                constraints: vec![],
                distributions: split_50_50(),
            }],
        },
        CreateExperimentRequest {
            key: "lt-onboarding-v2".into(),
            description: Some("Loadtest: onboarding v2".into()),
            primary_metric: "activation_rate".into(),
            variants: vec![control(), treatment(json!({"variant": "v2"}))],
            segments: vec![Segment {
                priority: 0,
                rollout_percent: 100,
                constraints: vec![Constraint {
                    property: "country".into(),
                    operator: ConstraintOperator::In,
                    value: json!(["US", "CA", "GB"]),
                }],
                distributions: split_50_50(),
            }],
        },
        CreateExperimentRequest {
            key: "lt-search-rank".into(),
            description: Some("Loadtest: search rank".into()),
            primary_metric: "click_through".into(),
            variants: vec![control(), treatment(json!({"algo": "v3"}))],
            segments: vec![
                Segment {
                    priority: 0,
                    rollout_percent: 100,
                    constraints: vec![Constraint {
                        property: "tier".into(),
                        operator: ConstraintOperator::Gte,
                        value: json!(2),
                    }],
                    distributions: vec![
                        Distribution { variant_key: "control".into(), percent: 30 },
                        Distribution { variant_key: "treatment".into(), percent: 70 },
                    ],
                },
                Segment {
                    priority: 1,
                    rollout_percent: 100,
                    constraints: vec![],
                    distributions: split_50_50(),
                },
            ],
        },
        CreateExperimentRequest {
            key: "lt-homepage-hero".into(),
            description: Some("Loadtest: homepage hero".into()),
            primary_metric: "engagement".into(),
            variants: vec![control(), treatment(json!({"layout": "hero-b"}))],
            segments: vec![Segment {
                priority: 0,
                rollout_percent: 25,
                constraints: vec![Constraint {
                    property: "country".into(),
                    operator: ConstraintOperator::Eq,
                    value: json!("US"),
                }],
                distributions: split_50_50(),
            }],
        },
    ]
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = get_config()?;

    let opts = SqliteConnectOptions::new()
        .filename(config.sqlite_filepath())
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePool::connect_with(opts).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    sqlx::query("DELETE FROM api_keys WHERE company_id = ?")
        .bind(COMPANY_ID)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM experiments WHERE company_id = ?")
        .bind(COMPANY_ID)
        .execute(&pool)
        .await?;

    let db = ExperimentsDB::new(pool);

    let key_name = format!("loadtest-{}", Utc::now().timestamp_millis());
    let key = api_key::create(&db, COMPANY_ID, key_name)
        .await
        .map_err(|e| format!("create api key: {e}"))?;

    let mut keys = Vec::new();
    for body in experiments() {
        let exp_key = body.key.clone();
        let id = experiment::create_experiment(&db, COMPANY_ID, body)
            .await
            .map_err(|e| format!("create experiment '{exp_key}': {e}"))?;
        experiment::start_experiment(&db, &id, COMPANY_ID)
            .await
            .map_err(|e| format!("start experiment '{exp_key}': {e}"))?;
        keys.push(exp_key);
    }

    std::fs::create_dir_all("loadtest")?;
    let env_path = std::path::PathBuf::from("loadtest").join("loadtest.env");
    std::fs::write(
        &env_path,
        format!(
            "export API_KEY={}\nexport EXPERIMENT_KEYS={}\n",
            key.plaintext,
            keys.join(",")
        ),
    )?;

    println!("seeded:");
    println!("  company_id      = {}", COMPANY_ID);
    println!("  api_key         = {}", key.plaintext);
    println!("  experiment_keys = {}", keys.join(","));
    println!("  wrote env file  = {}", env_path.display());
    println!();
    println!("note: if the server is running, restart it so the experiment cache picks up the fresh rows.");

    Ok(())
}
