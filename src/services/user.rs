use chrono::{Duration, Utc};

use crate::errors::CustomError;
use crate::models::{ExperimentsDB, UserListItem, UserRow};
use crate::repository::{
    db_create_pending_user, db_delete_user, db_list_company_users, CreatePendingUserOutcome,
    PendingUserInvite,
};
use crate::services::password::generate_invite_token;

pub fn normalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
}

pub struct InviteResult {
    pub user: UserRow,
    /// `None` when no invite token was issued (password provider disabled).
    pub plaintext_token: Option<String>,
}

/// Pre-create a stub `users` row in `company_id`. When `with_invite_token_ttl`
/// is `Some(days)`, also generate a one-time invite token that the new user can
/// trade for a password via `/auth/accept-invite` — the plaintext is returned
/// once in `InviteResult` and never stored. With `None`, the row is a pure
/// Google-only stub claimed by the first matching Google sign-in.
pub async fn invite(
    db: &ExperimentsDB,
    company_id: &str,
    email: &str,
    with_invite_token_ttl: Option<u32>,
) -> Result<InviteResult, CustomError> {
    let normalized = normalize_email(email);

    let token_and_expiry = with_invite_token_ttl.map(|days| {
        let generated = generate_invite_token();
        let expires_at = Utc::now()
            .checked_add_signed(Duration::days(days as i64))
            .expect("invite expiry within range")
            .timestamp_millis();
        (generated, expires_at)
    });

    let invite_arg = token_and_expiry.as_ref().map(|(g, exp)| PendingUserInvite {
        invite_token_hash: &g.hash,
        invite_token_expires_at: *exp,
    });

    match db_create_pending_user(db, company_id, &normalized, invite_arg).await? {
        CreatePendingUserOutcome::Created(row) => Ok(InviteResult {
            user: row,
            plaintext_token: token_and_expiry.map(|(g, _)| g.plaintext),
        }),
        CreatePendingUserOutcome::EmailExists => Err(CustomError::ConflictError(format!(
            "A user with email '{}' already exists",
            normalized
        ))),
    }
}

pub async fn list(
    db: &ExperimentsDB,
    company_id: &str,
) -> Result<Vec<UserListItem>, CustomError> {
    let rows = db_list_company_users(db, company_id).await?;
    Ok(rows.into_iter().map(UserListItem::from).collect())
}

pub async fn remove(
    db: &ExperimentsDB,
    user_id: &str,
    company_id: &str,
    actor_user_id: &str,
) -> Result<bool, CustomError> {
    if user_id == actor_user_id {
        return Err(CustomError::ForbiddenError(
            "You cannot remove yourself from the company".into(),
        ));
    }
    db_delete_user(db, user_id, company_id).await
}
