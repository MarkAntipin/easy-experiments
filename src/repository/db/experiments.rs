use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use serde_json::Value;
use sqlx::{self, QueryBuilder, Sqlite};
use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::{
    CachedExperiment, ExperimentRow, ExperimentStatus, ExperimentsDB, Segment, Variant,
};

/// Hard cap on a single experiment lookup. SQLite reads are normally sub-ms;
/// anything close to this means the connection is wedged and we'd rather
/// fail the request than tie up an actix worker indefinitely.
const EXPERIMENT_LOOKUP_TIMEOUT: Duration = Duration::from_secs(2);

/// Loader-side error for the moka cache. `Missing` is the "not found" signal
/// (deliberately not cached so we don't pin negative results — that's a
/// separate concern). `Failed` carries through real errors so callers see
/// them once the loader resolves.
#[derive(Debug)]
enum ExperimentLoadError {
    Missing,
    Failed(CustomError),
}

async fn invalidate_experiment_cache_by_id(db: &ExperimentsDB, id: &str, company_id: &str) {
    let key: Option<String> = sqlx::query_scalar(
        "
        SELECT
            key
        FROM experiments
        WHERE experiment_id = $1
          AND company_id = $2
        ",
    )
    .bind(id)
    .bind(company_id)
    .fetch_optional(&db.pool)
    .await
    .ok()
    .flatten();

    if let Some(k) = key {
        db.experiment_cache
            .invalidate(&(company_id.to_string(), k))
            .await;
    }
}

pub enum CreateExperimentOutcome {
    Created(String),
    KeyConflict,
}

pub async fn db_create_experiment(
    db: &ExperimentsDB,
    key: &str,
    description: Option<&str>,
    primary_metric: &str,
    variants: &[Variant],
    segments: &[Segment],
    company_id: &str,
) -> Result<CreateExperimentOutcome, CustomError> {
    let experiment_id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp_millis();
    let variants_json = serde_json::to_string(variants)
        .map_err(|e| CustomError::InternalError(format!("Failed to serialize variants: {}", e)))?;
    let segments_json = serde_json::to_string(segments)
        .map_err(|e| CustomError::InternalError(format!("Failed to serialize segments: {}", e)))?;

    let result = sqlx::query(
        "
        INSERT INTO experiments (
            experiment_id,
            key,
            description,
            status,
            primary_metric,
            variants,
            segments,
            created_at,
            updated_at,
            company_id
        )
        VALUES ($1, $2, $3, 'draft', $4, $5, $6, $7, $8, $9)
        ON CONFLICT (company_id, key) WHERE status != 'deleted' DO NOTHING
        ",
    )
    .bind(&experiment_id)
    .bind(key)
    .bind(description)
    .bind(primary_metric)
    .bind(&variants_json)
    .bind(&segments_json)
    .bind(now)
    .bind(now)
    .bind(company_id)
    .execute(&db.pool)
    .await?;

    if result.rows_affected() == 0 {
        Ok(CreateExperimentOutcome::KeyConflict)
    } else {
        Ok(CreateExperimentOutcome::Created(experiment_id))
    }
}

pub async fn db_get_experiment_by_id(
    db: &ExperimentsDB,
    id: &str,
    company_id: &str,
) -> Result<Option<ExperimentRow>, CustomError> {
    sqlx::query_as(
        "
        SELECT
            experiment_id,
            key,
            description,
            status,
            primary_metric,
            variants,
            segments,
            started_at,
            stopped_at,
            created_at,
            updated_at,
            company_id
        FROM experiments
        WHERE experiment_id = $1
          AND company_id = $2
          AND status != 'deleted'
        ",
    )
    .bind(id)
    .bind(company_id)
    .fetch_optional(&db.pool)
    .await
    .map_err(CustomError::from)
}

pub async fn db_get_experiment_by_key(
    db: &ExperimentsDB,
    key: &str,
    company_id: &str,
) -> Result<Option<Arc<CachedExperiment>>, CustomError> {
    let cache_key = (company_id.to_string(), key.to_string());
    let pool = db.pool.clone();
    let key_for_loader = key.to_string();
    let company_for_loader = company_id.to_string();

    // Single-flight: under a stampede of concurrent misses for the same key,
    // moka runs the loader exactly once and hands the result to all waiters.
    let result = db
        .experiment_cache
        .try_get_with::<_, ExperimentLoadError>(cache_key, async move {
            let query = sqlx::query_as(
                "
                SELECT
                    experiment_id,
                    key,
                    description,
                    status,
                    primary_metric,
                    variants,
                    segments,
                    started_at,
                    stopped_at,
                    created_at,
                    updated_at,
                    company_id
                FROM experiments
                WHERE key = $1
                  AND company_id = $2
                  AND status != 'deleted'
                ",
            )
            .bind(&key_for_loader)
            .bind(&company_for_loader)
            .fetch_optional(&pool);

            // Bound the SQLite call so a wedged connection can't pin an actix
            // worker. Timeout maps to a 500 today; the loader error is shared
            // with any concurrent waiters via moka's single-flight.
            let row: Option<ExperimentRow> = tokio::time::timeout(
                EXPERIMENT_LOOKUP_TIMEOUT,
                query,
            )
            .await
            .map_err(|_| {
                log::error!(
                    "experiment lookup timed out after {:?} for key={} company_id={}",
                    EXPERIMENT_LOOKUP_TIMEOUT,
                    key_for_loader,
                    company_for_loader,
                );
                ExperimentLoadError::Failed(CustomError::InternalError(
                    "experiment lookup timed out".to_string(),
                ))
            })?
            .map_err(|e| ExperimentLoadError::Failed(CustomError::from(e)))?;

            match row {
                None => Err(ExperimentLoadError::Missing),
                Some(r) => parse_cached_experiment(r)
                    .map(Arc::new)
                    .map_err(ExperimentLoadError::Failed),
            }
        })
        .await;

    match result {
        Ok(arc) => Ok(Some(arc)),
        Err(arc_err) => match arc_err.as_ref() {
            ExperimentLoadError::Missing => Ok(None),
            ExperimentLoadError::Failed(e) => Err(e.clone()),
        },
    }
}

fn parse_cached_experiment(row: ExperimentRow) -> Result<CachedExperiment, CustomError> {
    let variants: Vec<Variant> = serde_json::from_str(&row.variants).map_err(|e| {
        CustomError::InternalError(format!(
            "Failed to parse stored variants for experiment {}: {}",
            row.experiment_id, e
        ))
    })?;
    let mut segments: Vec<Segment> = serde_json::from_str(&row.segments).map_err(|e| {
        CustomError::InternalError(format!(
            "Failed to parse stored segments for experiment {}: {}",
            row.experiment_id, e
        ))
    })?;

    segments.sort_by_key(|s| s.priority);

    let variant_configs: HashMap<String, Arc<Value>> = variants
        .into_iter()
        .map(|v| (v.key, Arc::new(v.config)))
        .collect();

    Ok(CachedExperiment {
        experiment_id: row.experiment_id,
        status: row.status,
        variant_configs,
        segments,
    })
}

pub async fn db_get_experiments(
    db: &ExperimentsDB,
    status: Option<ExperimentStatus>,
    company_id: &str,
) -> Result<Vec<ExperimentRow>, CustomError> {
    match status {
        Some(status_filter) => {
            sqlx::query_as(
                "
                SELECT
                    experiment_id,
                    key,
                    description,
                    status,
                    primary_metric,
                    variants,
                    segments,
                    started_at,
                    stopped_at,
                    created_at,
                    updated_at,
                    company_id
                FROM experiments
                WHERE status = $1
                  AND company_id = $2
                ORDER BY updated_at DESC
                ",
            )
            .bind(status_filter.to_string())
            .bind(company_id)
            .fetch_all(&db.pool)
            .await
            .map_err(CustomError::from)
        }
        None => {
            sqlx::query_as(
                "
                SELECT
                    experiment_id,
                    key,
                    description,
                    status,
                    primary_metric,
                    variants,
                    segments,
                    started_at,
                    stopped_at,
                    created_at,
                    updated_at,
                    company_id
                FROM experiments
                WHERE company_id = $1
                  AND status != 'deleted'
                ORDER BY updated_at DESC
                ",
            )
            .bind(company_id)
            .fetch_all(&db.pool)
            .await
            .map_err(CustomError::from)
        }
    }
}

#[derive(Default)]
pub struct UpdateExperimentFields<'a> {
    /// `None` = don't touch; `Some(None)` = set to NULL; `Some(Some(v))` = set to v.
    pub description: Option<Option<&'a str>>,
    pub primary_metric: Option<&'a str>,
    pub variants: Option<&'a [Variant]>,
    pub segments: Option<&'a [Segment]>,
}

impl<'a> UpdateExperimentFields<'a> {
    pub fn is_empty(&self) -> bool {
        self.description.is_none()
            && self.primary_metric.is_none()
            && self.variants.is_none()
            && self.segments.is_none()
    }

    pub fn is_structural(&self) -> bool {
        self.primary_metric.is_some() || self.variants.is_some() || self.segments.is_some()
    }
}

pub enum UpdateExperimentOutcome {
    Updated,
    NotFound,
    StatusConflict(ExperimentStatus),
    VersionConflict,
}

pub async fn db_update_experiment(
    db: &ExperimentsDB,
    id: &str,
    company_id: &str,
    fields: UpdateExperimentFields<'_>,
    expected_updated_at: Option<i64>,
) -> Result<UpdateExperimentOutcome, CustomError> {
    if fields.is_empty() && expected_updated_at.is_none() {
        return Ok(UpdateExperimentOutcome::Updated);
    }

    let now = Utc::now().timestamp_millis();
    let structural = fields.is_structural();

    let variants_json = fields
        .variants
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| CustomError::InternalError(format!("Failed to serialize variants: {}", e)))?;
    let segments_json = fields
        .segments
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| CustomError::InternalError(format!("Failed to serialize segments: {}", e)))?;

    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new("UPDATE experiments SET updated_at = ");
    qb.push_bind(now);
    if let Some(desc) = fields.description {
        qb.push(", description = ");
        qb.push_bind(desc);
    }
    if let Some(pm) = fields.primary_metric {
        qb.push(", primary_metric = ");
        qb.push_bind(pm);
    }
    if let Some(v) = variants_json.as_ref() {
        qb.push(", variants = ");
        qb.push_bind(v);
    }
    if let Some(s) = segments_json.as_ref() {
        qb.push(", segments = ");
        qb.push_bind(s);
    }
    qb.push(" WHERE experiment_id = ");
    qb.push_bind(id);
    qb.push(" AND company_id = ");
    qb.push_bind(company_id);
    if structural {
        qb.push(" AND status = 'draft'");
    } else {
        qb.push(" AND status != 'deleted'");
    }
    if let Some(expected) = expected_updated_at {
        qb.push(" AND updated_at = ");
        qb.push_bind(expected);
    }

    let result = qb.build().execute(&db.pool).await?;

    if result.rows_affected() > 0 {
        invalidate_experiment_cache_by_id(db, id, company_id).await;
        return Ok(UpdateExperimentOutcome::Updated);
    }

    let existing: Option<(ExperimentStatus, i64)> = sqlx::query_as(
        "
        SELECT
            status,
            updated_at
        FROM experiments
        WHERE experiment_id = $1
          AND company_id = $2
          AND status != 'deleted'
        ",
    )
    .bind(id)
    .bind(company_id)
    .fetch_optional(&db.pool)
    .await?;

    match existing {
        None => Ok(UpdateExperimentOutcome::NotFound),
        Some((current_status, current_updated_at)) => {
            if let Some(expected) = expected_updated_at {
                if current_updated_at != expected {
                    return Ok(UpdateExperimentOutcome::VersionConflict);
                }
            }
            if structural && current_status != ExperimentStatus::Draft {
                return Ok(UpdateExperimentOutcome::StatusConflict(current_status));
            }
            // No real change occurred (e.g. same values). Treat as success.
            Ok(UpdateExperimentOutcome::Updated)
        }
    }
}

pub enum StartExperimentOutcome {
    Started,
    NotFound,
    NotInDraft(ExperimentStatus),
}

pub async fn db_start_experiment(
    db: &ExperimentsDB,
    id: &str,
    company_id: &str,
) -> Result<StartExperimentOutcome, CustomError> {
    let now = Utc::now().timestamp_millis();

    let result = sqlx::query(
        "
        UPDATE experiments
        SET
            status = 'running',
            started_at = $1,
            updated_at = $2
        WHERE experiment_id = $3
          AND company_id = $4
          AND status = 'draft'
        ",
    )
    .bind(now)
    .bind(now)
    .bind(id)
    .bind(company_id)
    .execute(&db.pool)
    .await?;

    if result.rows_affected() > 0 {
        invalidate_experiment_cache_by_id(db, id, company_id).await;
        return Ok(StartExperimentOutcome::Started);
    }

    let existing: Option<ExperimentStatus> = sqlx::query_scalar(
        "
        SELECT
            status
        FROM experiments
        WHERE experiment_id = $1
          AND company_id = $2
          AND status != 'deleted'
        ",
    )
    .bind(id)
    .bind(company_id)
    .fetch_optional(&db.pool)
    .await?;

    Ok(match existing {
        None => StartExperimentOutcome::NotFound,
        Some(status) => StartExperimentOutcome::NotInDraft(status),
    })
}

pub enum StopExperimentOutcome {
    Stopped,
    NotFound,
    NotRunning(ExperimentStatus),
}

pub async fn db_stop_experiment(
    db: &ExperimentsDB,
    id: &str,
    company_id: &str,
) -> Result<StopExperimentOutcome, CustomError> {
    let now = Utc::now().timestamp_millis();

    let result = sqlx::query(
        "
        UPDATE experiments
        SET
            status = 'stopped',
            stopped_at = $1,
            updated_at = $2
        WHERE experiment_id = $3
          AND company_id = $4
          AND status = 'running'
        ",
    )
    .bind(now)
    .bind(now)
    .bind(id)
    .bind(company_id)
    .execute(&db.pool)
    .await?;

    if result.rows_affected() > 0 {
        invalidate_experiment_cache_by_id(db, id, company_id).await;
        return Ok(StopExperimentOutcome::Stopped);
    }

    let existing: Option<ExperimentStatus> = sqlx::query_scalar(
        "
        SELECT
            status
        FROM experiments
        WHERE experiment_id = $1
          AND company_id = $2
          AND status != 'deleted'
        ",
    )
    .bind(id)
    .bind(company_id)
    .fetch_optional(&db.pool)
    .await?;

    Ok(match existing {
        None => StopExperimentOutcome::NotFound,
        Some(status) => StopExperimentOutcome::NotRunning(status),
    })
}

pub enum DeleteExperimentOutcome {
    Deleted,
    NotFound,
    NotDeletable(ExperimentStatus),
}

pub async fn db_delete_experiment(
    db: &ExperimentsDB,
    id: &str,
    company_id: &str,
) -> Result<DeleteExperimentOutcome, CustomError> {
    let now = Utc::now().timestamp_millis();

    let mut tx = db.pool.begin().await?;

    // RETURNING key in the same statement so cache invalidation never depends
    // on a second post-commit lookup that could silently fail and leave eval
    // serving the deleted row until TTL.
    let updated_key: Option<String> = sqlx::query_scalar(
        "
        UPDATE experiments
        SET
            status = 'deleted',
            updated_at = $1
        WHERE experiment_id = $2
          AND company_id = $3
          AND status IN ('draft', 'stopped')
        RETURNING key
        ",
    )
    .bind(now)
    .bind(id)
    .bind(company_id)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(key) = updated_key {
        tx.commit().await?;
        db.experiment_cache
            .invalidate(&(company_id.to_string(), key))
            .await;
        return Ok(DeleteExperimentOutcome::Deleted);
    }

    let existing: Option<ExperimentStatus> = sqlx::query_scalar(
        "
        SELECT
            status
        FROM experiments
        WHERE experiment_id = $1
          AND company_id = $2
          AND status != 'deleted'
        ",
    )
    .bind(id)
    .bind(company_id)
    .fetch_optional(&mut *tx)
    .await?;

    // No write occurred; let the tx drop (auto-rollback) instead of a commit
    // whose error would surface as 500 on what is logically a 404/409.
    drop(tx);

    Ok(match existing {
        None => DeleteExperimentOutcome::NotFound,
        Some(status) => DeleteExperimentOutcome::NotDeletable(status),
    })
}
