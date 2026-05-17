//! Email + password auth. Lives alongside `services::auth` (Google) and produces
//! the same `LoginResponse` shape so callers downstream of `/auth/*` can treat
//! both flows identically.
//!
//! The two entry points:
//!   * `password_login` — proves identity against a stored argon2 hash.
//!   * `accept_invite`  — consumes a one-time invite token (created by the
//!                        existing admin invite flow), sets the user's password,
//!                        and mints a JWT in one shot.
//!
//! Argon2 verification is intentionally slow (~50-100ms), but it only runs on
//! login / accept-invite, never on the SDK hot path.

use chrono::Utc;

use crate::errors::CustomError;
use crate::models::{AuthenticatedUser, CompanyRow, ExperimentsDB, UserRow};
use crate::repository::{
    db_find_user_by_email, db_find_user_by_invite_token_hash, db_set_password_and_clear_invite,
};
use crate::services::jwt::create_jwt;
use crate::services::password::{
    hash_invite_token, hash_password, validate_password_strength, verify_password,
};
use crate::services::user::normalize_email;

pub struct PasswordAuthResult {
    pub token: String,
    pub user: UserRow,
    pub company: CompanyRow,
}

pub async fn password_login(
    db: &ExperimentsDB,
    jwt_secret: &str,
    email: &str,
    password: &str,
) -> Result<PasswordAuthResult, CustomError> {
    // Generic 401 for any failure mode below — never reveal whether the email
    // exists, whether the user is a Google-only account, or which field is wrong.
    let invalid = || CustomError::UnauthorizedError("invalid email or password".into());

    let normalized = normalize_email(email);
    let Some((user, company)) = db_find_user_by_email(db, &normalized).await? else {
        return Err(invalid());
    };
    let Some(stored_hash) = user.password_hash.as_ref() else {
        return Err(invalid());
    };
    if !verify_password(password, stored_hash)? {
        return Err(invalid());
    }

    let token = mint_jwt(&user, jwt_secret)?;
    Ok(PasswordAuthResult {
        token,
        user,
        company,
    })
}

pub async fn accept_invite(
    db: &ExperimentsDB,
    jwt_secret: &str,
    invite_token: &str,
    new_password: &str,
) -> Result<PasswordAuthResult, CustomError> {
    validate_password_strength(new_password)?;

    let token_hash = hash_invite_token(invite_token);
    let user = db_find_user_by_invite_token_hash(db, &token_hash)
        .await?
        .ok_or_else(|| {
            CustomError::UnauthorizedError("invite link is invalid or already used".into())
        })?;

    // A row may exist but its expiry has passed. Treat it like a missing token
    // — same generic message, no leaking of whether the token existed at all.
    let now = Utc::now().timestamp_millis();
    let expires_at = user
        .invite_token_expires_at
        .ok_or_else(|| CustomError::InternalError("invite row missing expiry".into()))?;
    if expires_at <= now {
        return Err(CustomError::UnauthorizedError(
            "invite link is invalid or already used".into(),
        ));
    }

    // Refuse to overwrite an already-claimed account. The token uniqueness +
    // clearing on accept means this branch shouldn't normally fire, but if
    // someone resets the token onto an existing user it protects them.
    if user.password_hash.is_some() {
        return Err(CustomError::ConflictError(
            "account already has a password — sign in instead".into(),
        ));
    }

    let password_hash = hash_password(new_password)?;
    db_set_password_and_clear_invite(db, &user.user_id, &password_hash).await?;

    let mut claimed = user;
    claimed.password_hash = Some(password_hash);
    claimed.invite_token_hash = None;
    claimed.invite_token_expires_at = None;

    let company = db_find_user_by_email(db, &claimed.email)
        .await?
        .map(|(_, c)| c)
        .ok_or_else(|| CustomError::InternalError("company vanished mid-accept".into()))?;

    let token = mint_jwt(&claimed, jwt_secret)?;
    Ok(PasswordAuthResult {
        token,
        user: claimed,
        company,
    })
}

fn mint_jwt(user: &UserRow, jwt_secret: &str) -> Result<String, CustomError> {
    let auth_user = AuthenticatedUser {
        user_id: user.user_id.clone(),
        company_id: user.company_id.clone(),
        email: user.email.clone(),
        role: user.role,
    };
    create_jwt(&auth_user, jwt_secret)
}
