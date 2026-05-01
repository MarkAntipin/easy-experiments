use chrono::Utc;
use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::{ApiKeyRow, ExperimentsDB};

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
) -> Result<Option<ApiKeyRow>, CustomError> {
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
        WHERE key_hash = $1
        ",
    )
    .bind(key_hash)
    .fetch_optional(&db.pool)
    .await
    .map_err(CustomError::from)
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
    let result = sqlx::query(
        "
        DELETE FROM api_keys
        WHERE api_key_id = $1
          AND company_id = $2
        ",
    )
    .bind(api_key_id)
    .bind(company_id)
    .execute(&db.pool)
    .await
    .map_err(CustomError::from)?;

    Ok(result.rows_affected() > 0)
}
