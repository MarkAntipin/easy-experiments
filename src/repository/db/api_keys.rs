use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::{ApiKeyAuthRow, ApiKeyListRow, ExperimentsDB};

#[derive(Debug)]
enum ApiKeyLoadError {
    Missing,
    Failed(CustomError),
}

pub struct NewApiKey<'a> {
    pub company_id: &'a str,
    pub name: &'a str,
    pub key_hash: &'a str,
    pub key_prefix: &'a str,
}

pub enum CreateApiKeyOutcome {
    Created { api_key_id: String, created_at: i64 },
    NameConflict,
}

pub async fn db_create_api_key(
    db: &ExperimentsDB,
    new: NewApiKey<'_>,
) -> Result<CreateApiKeyOutcome, CustomError> {
    let api_key_id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp_millis();

    let result = sqlx::query(
        "
        INSERT INTO api_keys (
            api_key_id,
            company_id,
            name,
            key_hash,
            key_prefix,
            created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (company_id, name) DO NOTHING
        ",
    )
    .bind(&api_key_id)
    .bind(new.company_id)
    .bind(new.name)
    .bind(new.key_hash)
    .bind(new.key_prefix)
    .bind(now)
    .execute(&db.pool)
    .await
    .map_err(CustomError::from)?;

    if result.rows_affected() == 0 {
        Ok(CreateApiKeyOutcome::NameConflict)
    } else {
        Ok(CreateApiKeyOutcome::Created {
            api_key_id,
            created_at: now,
        })
    }
}

pub async fn db_count_api_keys(
    db: &ExperimentsDB,
    company_id: &str,
) -> Result<i64, CustomError> {
    sqlx::query_scalar("SELECT COUNT(*) FROM api_keys WHERE company_id = $1")
        .bind(company_id)
        .fetch_one(&db.pool)
        .await
        .map_err(CustomError::from)
}

pub async fn db_find_api_key_by_hash(
    db: &ExperimentsDB,
    key_hash: &str,
) -> Result<Option<Arc<ApiKeyAuthRow>>, CustomError> {
    let pool = db.pool.clone();
    let hash_for_loader = key_hash.to_string();

    // Single-flight: concurrent misses for the same hash all wait on one
    // SQLite query rather than stampeding the pool.
    let result = db
        .api_key_cache
        .try_get_with::<_, ApiKeyLoadError>(key_hash.to_string(), async move {
            let row: Option<ApiKeyAuthRow> = sqlx::query_as(
                "
                SELECT
                    api_key_id,
                    company_id
                FROM api_keys
                WHERE key_hash = $1
                ",
            )
            .bind(&hash_for_loader)
            .fetch_optional(&pool)
            .await
            .map_err(|e| ApiKeyLoadError::Failed(CustomError::from(e)))?;

            match row {
                None => Err(ApiKeyLoadError::Missing),
                Some(r) => Ok(Arc::new(r)),
            }
        })
        .await;

    match result {
        Ok(arc) => Ok(Some(arc)),
        Err(arc_err) => match arc_err.as_ref() {
            ApiKeyLoadError::Missing => Ok(None),
            ApiKeyLoadError::Failed(e) => Err(e.clone()),
        },
    }
}

pub async fn db_list_api_key_summaries(
    db: &ExperimentsDB,
    company_id: &str,
) -> Result<Vec<ApiKeyListRow>, CustomError> {
    sqlx::query_as(
        "
        SELECT
            api_key_id,
            name,
            key_prefix,
            created_at
        FROM api_keys
        WHERE company_id = $1
        ORDER BY created_at DESC, api_key_id ASC
        ",
    )
    .bind(company_id)
    .fetch_all(&db.pool)
    .await
    .map_err(CustomError::from)
}

pub async fn db_revoke_api_key(
    db: &ExperimentsDB,
    api_key_id: &str,
    company_id: &str,
) -> Result<bool, CustomError> {
    // RETURNING the hash so we can invalidate the auth cache; otherwise a
    // revoked key would still authenticate until the TTL expires.
    let row: Option<(String,)> = sqlx::query_as(
        "
        DELETE FROM api_keys
        WHERE api_key_id = $1
          AND company_id = $2
        RETURNING key_hash
        ",
    )
    .bind(api_key_id)
    .bind(company_id)
    .fetch_optional(&db.pool)
    .await
    .map_err(CustomError::from)?;

    let Some((hash,)) = row else {
        return Ok(false);
    };
    db.api_key_cache.invalidate(&hash).await;
    Ok(true)
}
