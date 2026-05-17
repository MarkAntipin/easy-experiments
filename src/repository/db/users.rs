use chrono::Utc;
use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::{CompanyRow, ExperimentsDB, UserRole, UserRow};

const USER_COLS: &str = "user_id, company_id, email, name, picture_url, google_sub, password_hash, invite_token_hash, invite_token_expires_at, role, created_at, updated_at";

pub async fn db_find_user_by_google_sub(
    db: &ExperimentsDB,
    google_sub: &str,
) -> Result<Option<(UserRow, CompanyRow)>, CustomError> {
    let user: Option<UserRow> = sqlx::query_as(&format!(
        "SELECT {USER_COLS} FROM users WHERE google_sub = $1"
    ))
    .bind(google_sub)
    .fetch_optional(&db.pool)
    .await
    .map_err(CustomError::from)?;

    match user {
        Some(u) => {
            let company = fetch_company(db, &u.company_id).await?;
            Ok(Some((u, company)))
        }
        None => Ok(None),
    }
}

pub async fn db_find_user_by_email(
    db: &ExperimentsDB,
    email: &str,
) -> Result<Option<(UserRow, CompanyRow)>, CustomError> {
    let user: Option<UserRow> =
        sqlx::query_as(&format!("SELECT {USER_COLS} FROM users WHERE email = $1"))
            .bind(email)
            .fetch_optional(&db.pool)
            .await
            .map_err(CustomError::from)?;

    match user {
        Some(u) => {
            let company = fetch_company(db, &u.company_id).await?;
            Ok(Some((u, company)))
        }
        None => Ok(None),
    }
}

async fn fetch_company(db: &ExperimentsDB, company_id: &str) -> Result<CompanyRow, CustomError> {
    sqlx::query_as(
        "SELECT company_id, name, created_at, updated_at FROM companies WHERE company_id = $1",
    )
    .bind(company_id)
    .fetch_one(&db.pool)
    .await
    .map_err(CustomError::from)
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
            role,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ",
    )
    .bind(&user_id)
    .bind(&company_id)
    .bind(email)
    .bind(name)
    .bind(picture_url)
    .bind(google_sub)
    .bind(UserRole::Admin)
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
        google_sub: Some(google_sub.to_string()),
        password_hash: None,
        invite_token_hash: None,
        invite_token_expires_at: None,
        role: UserRole::Admin,
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

#[allow(clippy::large_enum_variant)]
pub enum CreatePendingUserOutcome {
    Created(UserRow),
    EmailExists,
}

pub struct PendingUserInvite<'a> {
    pub invite_token_hash: &'a str,
    pub invite_token_expires_at: i64,
}

pub async fn db_create_pending_user(
    db: &ExperimentsDB,
    company_id: &str,
    email: &str,
    invite: Option<PendingUserInvite<'_>>,
) -> Result<CreatePendingUserOutcome, CustomError> {
    let user_id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp_millis();

    let result = sqlx::query(
        "
        INSERT INTO users (
            user_id,
            company_id,
            email,
            name,
            picture_url,
            google_sub,
            password_hash,
            invite_token_hash,
            invite_token_expires_at,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, NULL, NULL, NULL, NULL, $4, $5, $6, $6)
        ON CONFLICT (email) DO NOTHING
        ",
    )
    .bind(&user_id)
    .bind(company_id)
    .bind(email)
    .bind(invite.as_ref().map(|i| i.invite_token_hash))
    .bind(invite.as_ref().map(|i| i.invite_token_expires_at))
    .bind(now)
    .execute(&db.pool)
    .await
    .map_err(CustomError::from)?;

    if result.rows_affected() == 0 {
        return Ok(CreatePendingUserOutcome::EmailExists);
    }

    Ok(CreatePendingUserOutcome::Created(UserRow {
        user_id,
        company_id: company_id.to_string(),
        email: email.to_string(),
        name: None,
        picture_url: None,
        google_sub: None,
        password_hash: None,
        invite_token_hash: invite.as_ref().map(|i| i.invite_token_hash.to_string()),
        invite_token_expires_at: invite.as_ref().map(|i| i.invite_token_expires_at),
        role: UserRole::Member,
        created_at: now,
        updated_at: now,
    }))
}

pub async fn db_find_user_by_invite_token_hash(
    db: &ExperimentsDB,
    token_hash: &str,
) -> Result<Option<UserRow>, CustomError> {
    sqlx::query_as(&format!(
        "SELECT {USER_COLS} FROM users WHERE invite_token_hash = $1"
    ))
    .bind(token_hash)
    .fetch_optional(&db.pool)
    .await
    .map_err(CustomError::from)
}

pub async fn db_set_password_and_clear_invite(
    db: &ExperimentsDB,
    user_id: &str,
    password_hash: &str,
) -> Result<(), CustomError> {
    let now = Utc::now().timestamp_millis();
    sqlx::query(
        "
        UPDATE users
        SET password_hash = $1,
            invite_token_hash = NULL,
            invite_token_expires_at = NULL,
            updated_at = $2
        WHERE user_id = $3
        ",
    )
    .bind(password_hash)
    .bind(now)
    .bind(user_id)
    .execute(&db.pool)
    .await
    .map_err(CustomError::from)?;
    Ok(())
}

pub async fn db_count_users(db: &ExperimentsDB) -> Result<i64, CustomError> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&db.pool)
        .await
        .map_err(CustomError::from)?;
    Ok(row.0)
}

pub async fn db_create_password_admin_and_company(
    db: &ExperimentsDB,
    email: &str,
    company_name: &str,
    password_hash: &str,
) -> Result<(UserRow, CompanyRow), CustomError> {
    let now = Utc::now().timestamp_millis();
    let company_id = Uuid::new_v4().to_string();
    let user_id = Uuid::new_v4().to_string();

    let mut tx = db.pool.begin().await.map_err(CustomError::from)?;

    sqlx::query(
        "INSERT INTO companies (company_id, name, created_at, updated_at) VALUES ($1, $2, $3, $4)",
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
            user_id, company_id, email, name, picture_url,
            google_sub, password_hash,
            invite_token_hash, invite_token_expires_at,
            role, created_at, updated_at
        )
        VALUES ($1, $2, $3, NULL, NULL, NULL, $4, NULL, NULL, $5, $6, $6)
        ",
    )
    .bind(&user_id)
    .bind(&company_id)
    .bind(email)
    .bind(password_hash)
    .bind(UserRole::Admin)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(CustomError::from)?;

    tx.commit().await.map_err(CustomError::from)?;

    Ok((
        UserRow {
            user_id,
            company_id: company_id.clone(),
            email: email.to_string(),
            name: None,
            picture_url: None,
            google_sub: None,
            password_hash: Some(password_hash.to_string()),
            invite_token_hash: None,
            invite_token_expires_at: None,
            role: UserRole::Admin,
            created_at: now,
            updated_at: now,
        },
        CompanyRow {
            company_id,
            name: company_name.to_string(),
            created_at: now,
            updated_at: now,
        },
    ))
}

pub async fn db_bind_user_google_sub(
    db: &ExperimentsDB,
    user_id: &str,
    google_sub: &str,
) -> Result<(), CustomError> {
    let now = Utc::now().timestamp_millis();
    sqlx::query(
        "
        UPDATE users
        SET google_sub = $1,
            updated_at = $2
        WHERE user_id = $3
        ",
    )
    .bind(google_sub)
    .bind(now)
    .bind(user_id)
    .execute(&db.pool)
    .await
    .map_err(CustomError::from)?;
    Ok(())
}

pub async fn db_list_company_users(
    db: &ExperimentsDB,
    company_id: &str,
) -> Result<Vec<UserRow>, CustomError> {
    sqlx::query_as(&format!(
        "SELECT {USER_COLS} FROM users \
         WHERE company_id = $1 \
         ORDER BY created_at ASC, user_id ASC"
    ))
    .bind(company_id)
    .fetch_all(&db.pool)
    .await
    .map_err(CustomError::from)
}

pub async fn db_delete_user(
    db: &ExperimentsDB,
    user_id: &str,
    company_id: &str,
) -> Result<bool, CustomError> {
    let rows = sqlx::query("DELETE FROM users WHERE user_id = $1 AND company_id = $2")
        .bind(user_id)
        .bind(company_id)
        .execute(&db.pool)
        .await
        .map_err(CustomError::from)?;
    Ok(rows.rows_affected() > 0)
}

pub async fn db_fetch_user_role(
    db: &ExperimentsDB,
    user_id: &str,
    company_id: &str,
) -> Result<Option<UserRole>, CustomError> {
    let row: Option<(UserRole,)> =
        sqlx::query_as("SELECT role FROM users WHERE user_id = $1 AND company_id = $2")
            .bind(user_id)
            .bind(company_id)
            .fetch_optional(&db.pool)
            .await
            .map_err(CustomError::from)?;
    Ok(row.map(|(r,)| r))
}
