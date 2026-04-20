use sqlx;
use crate::errors::CustomError;
use crate::models::{ExperimentDBRow, ExperimentsDB};
use crate::enums::ExperimentStatus;
use chrono::Utc;

pub async fn db_create_experiment(
    db: &ExperimentsDB,
    row: &ExperimentDBRow,
) -> Result<Option<i64>, sqlx::Error> {
    let status_str = row.status.to_string();

    let result = sqlx::query!(
        r#"
        INSERT INTO experiments (name, description, status, variants, traffic_percentage, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT(name) DO NOTHING
        "#,
        row.name,
        row.description,
        status_str,
        row.variants,
        row.traffic_percentage,
        row.created_at,
        row.updated_at
    )
    .execute(&db.pool)
    .await?;

    if result.rows_affected() > 0 {
        let id_row = sqlx::query!("SELECT last_insert_rowid() as id")
            .fetch_one(&db.pool)
            .await?;
        Ok(Some(id_row.id as i64))
    } else {
        Ok(None)
    }
}

pub async fn db_get_experiment_by_id(
    db: &ExperimentsDB,
    id: i64,
) -> Result<Option<ExperimentDBRow>, sqlx::Error> {
    let row: Option<ExperimentDBRow> = sqlx::query_as!(
        ExperimentDBRow,
        r#"
        SELECT
            id as "id!",
            name as "name!",
            description as "description!",
            status as "status!: _",
            variants as "variants!",
            traffic_percentage as "traffic_percentage!",
            created_at as "created_at!: _",
            updated_at as "updated_at!: _"
        FROM experiments
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&db.pool)
    .await?;

    Ok(row)
}

pub async fn db_get_experiments(
    db: &ExperimentsDB,
    status: Option<String>,
) -> Result<Vec<ExperimentDBRow>, sqlx::Error> {
    let rows = match status {
        Some(status_filter) => {
            sqlx::query_as!(
                ExperimentDBRow,
                r#"
                SELECT
                    id as "id!",
                    name as "name!",
                    description as "description!",
                    status as "status!: _",
                    variants as "variants!",
                    traffic_percentage as "traffic_percentage!",
                    created_at as "created_at!: _",
                    updated_at as "updated_at!: _"
                FROM experiments
                WHERE status = $1
                ORDER BY created_at DESC
                "#,
                status_filter
            )
            .fetch_all(&db.pool)
            .await?
        }
        None => {
            sqlx::query_as!(
                ExperimentDBRow,
                r#"
                SELECT
                    id as "id!",
                    name as "name!",
                    description as "description!",
                    status as "status!: _",
                    variants as "variants!",
                    traffic_percentage as "traffic_percentage!",
                    created_at as "created_at!: _",
                    updated_at as "updated_at!: _"
                FROM experiments
                ORDER BY created_at DESC
                "#,
            )
            .fetch_all(&db.pool)
            .await?
        }
    };
    Ok(rows)
}

pub async fn db_update_experiment(
    db: &ExperimentsDB,
    id: i64,
    name: Option<&str>,
    description: Option<&str>,
    variants: Option<&str>,
    traffic_percentage: Option<f64>,
) -> Result<Option<i64>, CustomError> {
    let existing = db_get_experiment_by_id(db, id).await.map_err(CustomError::from)?;
    if existing.is_none() {
        return Ok(None);
    }
    let existing = existing.unwrap();

    let new_name = name.unwrap_or(&existing.name);
    let new_description = description.unwrap_or(&existing.description);
    let new_variants = variants.unwrap_or(&existing.variants);
    let new_traffic = traffic_percentage.unwrap_or(existing.traffic_percentage);
    let now = Utc::now();

    sqlx::query!(
        r#"
        UPDATE experiments
        SET name = $1, description = $2, variants = $3, traffic_percentage = $4, updated_at = $5
        WHERE id = $6
        "#,
        new_name,
        new_description,
        new_variants,
        new_traffic,
        now,
        id
    )
    .execute(&db.pool)
    .await
    .map_err(CustomError::from)?;

    Ok(Some(id))
}

pub async fn db_update_experiment_status(
    db: &ExperimentsDB,
    id: i64,
    status: ExperimentStatus,
) -> Result<Option<i64>, CustomError> {
    let existing = db_get_experiment_by_id(db, id).await.map_err(CustomError::from)?;
    if existing.is_none() {
        return Ok(None);
    }

    let status_str = status.to_string();
    let now = Utc::now();

    sqlx::query!(
        r#"
        UPDATE experiments
        SET status = $1, updated_at = $2
        WHERE id = $3
        "#,
        status_str,
        now,
        id
    )
    .execute(&db.pool)
    .await
    .map_err(CustomError::from)?;

    Ok(Some(id))
}

pub async fn db_delete_experiment(
    db: &ExperimentsDB,
    id: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM experiments WHERE id = $1",
        id
    )
    .execute(&db.pool)
    .await?;

    Ok(result.rows_affected() > 0)
}
