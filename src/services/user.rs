use crate::errors::CustomError;
use crate::models::{ExperimentsDB, UserListItem, UserRow};
use crate::repository::{
    db_create_pending_user, db_delete_user, db_list_company_users, CreatePendingUserOutcome,
};

pub fn normalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
}

/// Pre-create a stub `users` row in `company_id`. The row has `google_sub = NULL`
/// until the invitee signs in with Google for the first time, at which point
/// `auth::provision_and_mint` claims the stub by binding the real sub.
pub async fn invite(
    db: &ExperimentsDB,
    company_id: &str,
    email: &str,
) -> Result<UserRow, CustomError> {
    let normalized = normalize_email(email);
    match db_create_pending_user(db, company_id, &normalized).await? {
        CreatePendingUserOutcome::Created(row) => Ok(row),
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
