use serde::{Deserialize, Serialize};

use crate::errors::CustomError;
use crate::models::{UserRole, UserRow};
use crate::validation::Validate;

const MAX_EMAIL_LENGTH: usize = 256;

#[derive(Serialize, Deserialize)]
pub struct InviteUserRequest {
    pub email: String,
}

impl Validate for InviteUserRequest {
    fn validate(&self) -> Result<(), CustomError> {
        let trimmed = self.email.trim();
        if trimmed.is_empty() {
            return Err(CustomError::ValidationError(
                "email should not be empty".into(),
            ));
        }
        if trimmed.len() > MAX_EMAIL_LENGTH {
            return Err(CustomError::ValidationError(format!(
                "email length should be less than {} bytes",
                MAX_EMAIL_LENGTH
            )));
        }
        // Cheap structural check — full RFC-5322 is more grief than it's worth
        // at this layer, and the only way to truly verify an address is to use
        // it. We just keep out obvious typos.
        let at_count = trimmed.matches('@').count();
        if at_count != 1 {
            return Err(CustomError::ValidationError(
                "email must contain exactly one '@'".into(),
            ));
        }
        let (local, domain) = trimmed.split_once('@').unwrap();
        if local.is_empty() || domain.is_empty() || !domain.contains('.') {
            return Err(CustomError::ValidationError("email is not valid".into()));
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Pending,
    Active,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserListItem {
    pub user_id: String,
    pub email: String,
    pub name: Option<String>,
    pub picture_url: Option<String>,
    pub status: UserStatus,
    pub role: UserRole,
    pub created_at: i64,
}

impl From<UserRow> for UserListItem {
    fn from(row: UserRow) -> Self {
        let status = if row.google_sub.is_some() {
            UserStatus::Active
        } else {
            UserStatus::Pending
        };
        Self {
            user_id: row.user_id,
            email: row.email,
            name: row.name,
            picture_url: row.picture_url,
            status,
            role: row.role,
            created_at: row.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct UserListResponse {
    pub items: Vec<UserListItem>,
}
