use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::{ApiKeyRow, ExperimentsDB};

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

pub async fn db_create_api_key(
    db: &ExperimentsDB,
    new: NewApiKey<'_>,
) -> Result<(String, i64), CustomError> {
    let api_key_id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp_millis();

    sqlx::query(
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

    Ok((api_key_id, now))
}

pub async fn db_find_api_key_by_hash(
    db: &ExperimentsDB,
    key_hash: &str,
) -> Result<Option<Arc<ApiKeyRow>>, CustomError> {
    let pool = db.pool.clone();
    let hash_for_loader = key_hash.to_string();

    // Single-flight: concurrent misses for the same hash all wait on one
    // SQLite query rather than stampeding the pool.
    let result = db
        .api_key_cache
        .try_get_with::<_, ApiKeyLoadError>(key_hash.to_string(), async move {
            let row: Option<ApiKeyRow> = sqlx::query_as(
                "
                SELECT
                    api_key_id,
                    company_id,
                    name,
                    key_hash,
                    key_prefix,
                    created_at
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

pub async fn db_list_api_keys(
    db: &ExperimentsDB,
    company_id: &str,
) -> Result<Vec<ApiKeyRow>, CustomError> {
    sqlx::query_as(
        "
        SELECT
            api_key_id,
            company_id,
            name,
            key_hash,
            key_prefix,
            created_at
        FROM api_keys
        WHERE company_id = $1
        ORDER BY created_at DESC
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
