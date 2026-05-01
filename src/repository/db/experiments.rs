use std::sync::Arc;

use chrono::Utc;
use sqlx::{self, QueryBuilder, Sqlite};
use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::{
    CachedExperiment, ExperimentRow, ExperimentStatus, ExperimentsDB, Segment, Variant,
};

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

    if let Some(cached) = db.experiment_cache.get(&cache_key).await {
        return Ok(Some(cached));
    }

    let row: Option<ExperimentRow> = sqlx::query_as(
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
    .bind(key)
    .bind(company_id)
    .fetch_optional(&db.pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let cached = Arc::new(parse_cached_experiment(row)?);
    db.experiment_cache.insert(cache_key, Arc::clone(&cached)).await;
    Ok(Some(cached))
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

    Ok(CachedExperiment {
        experiment_id: row.experiment_id,
        status: row.status,
        variants,
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

    let result = sqlx::query(
        "
        UPDATE experiments
        SET
            status = 'deleted',
            updated_at = $1
        WHERE experiment_id = $2
          AND company_id = $3
          AND status IN ('draft', 'stopped')
        ",
    )
    .bind(now)
    .bind(id)
    .bind(company_id)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() > 0 {
        tx.commit().await?;
        invalidate_experiment_cache_by_id(db, id, company_id).await;
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

    tx.commit().await?;

    Ok(match existing {
        None => DeleteExperimentOutcome::NotFound,
        Some(status) => DeleteExperimentOutcome::NotDeletable(status),
    })
}
