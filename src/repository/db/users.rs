use chrono::Utc;
use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::{CompanyRow, ExperimentsDB, UserRow};

pub async fn db_find_user_by_google_sub(
    db: &ExperimentsDB,
    google_sub: &str,
) -> Result<Option<(UserRow, CompanyRow)>, CustomError> {
    let user: Option<UserRow> = sqlx::query_as(
        "
        SELECT
            user_id,
            company_id,
            email,
            name,
            picture_url,
            google_sub,
            created_at,
            updated_at
        FROM users
        WHERE google_sub = $1
        ",
    )
    .bind(google_sub)
    .fetch_optional(&db.pool)
    .await
    .map_err(CustomError::from)?;

    match user {
        Some(u) => {
            let company: CompanyRow = sqlx::query_as(
                "
                SELECT
                    company_id,
                    name,
                    created_at,
                    updated_at
                FROM companies
                WHERE company_id = $1
                ",
            )
            .bind(&u.company_id)
            .fetch_one(&db.pool)
            .await
            .map_err(CustomError::from)?;
            Ok(Some((u, company)))
        }
        None => Ok(None),
    }
}

pub async fn db_update_user_profile(
    db: &ExperimentsDB,
    user_id: &str,
    name: Option<&str>,
    picture_url: Option<&str>,
) -> Result<(), CustomError> {
    let now = Utc::now().timestamp_millis();

    sqlx::query(
        "
        UPDATE users
        SET
            name = $1,
            picture_url = $2,
            updated_at = $3
        WHERE user_id = $4
        ",
    )
    .bind(name)
    .bind(picture_url)
    .bind(now)
    .bind(user_id)
    .execute(&db.pool)
    .await
    .map_err(CustomError::from)?;

    Ok(())
}

pub async fn db_create_user_and_company(
    db: &ExperimentsDB,
    email: &str,
    company_name: &str,
    name: Option<&str>,
    picture_url: Option<&str>,
    google_sub: &str,
) -> Result<(UserRow, CompanyRow), CustomError> {
    let now = Utc::now().timestamp_millis();
    let company_id = Uuid::new_v4().to_string();
    let user_id = Uuid::new_v4().to_string();

    let mut tx = db.pool.begin().await.map_err(CustomError::from)?;

    sqlx::query(
        "
        INSERT INTO companies (
            company_id,
            name,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4)
        ",
    )
    .bind(&company_id)
    .bind(company_name)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(CustomError::from)?;

    sqlx::query(
        "
        INSERT INTO users (
            user_id,
            company_id,
            email,
            name,
            picture_url,
            google_sub,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ",
    )
    .bind(&user_id)
    .bind(&company_id)
    .bind(email)
    .bind(name)
    .bind(picture_url)
    .bind(google_sub)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(CustomError::from)?;

    tx.commit().await.map_err(CustomError::from)?;

    let user = UserRow {
        user_id,
        company_id: company_id.clone(),
        email: email.to_string(),
        name: name.map(str::to_string),
        picture_url: picture_url.map(str::to_string),
        google_sub: google_sub.to_string(),
        created_at: now,
        updated_at: now,
    };

    let company = CompanyRow {
        company_id,
        name: company_name.to_string(),
        created_at: now,
        updated_at: now,
    };

    Ok((user, company))
}
