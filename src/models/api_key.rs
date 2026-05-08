use std::sync::Arc;

use moka::future::Cache;
use serde::{Deserialize, Serialize};

use crate::errors::CustomError;
use crate::validation::Validate;

/// Cache value is `Option<Arc<...>>` so both hits *and* misses memoize.
/// Without negative caching, an unauthenticated client can spray random
/// `X-Api-Key` headers and force one SQLite query per request.
pub type ApiKeyCache = Cache<String, Option<Arc<ApiKeyAuthRow>>>;

#[derive(Clone, Debug)]
pub struct AuthenticatedApiKey {
    pub api_key_id: String,
    pub company_id: String,
}

#[derive(sqlx::FromRow)]
pub struct ApiKeyAuthRow {
    pub api_key_id: String,
    pub company_id: String,
}

const MAX_API_KEY_NAME_LENGTH: usize = 128;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyRequest {
    pub name: String,
}

impl Validate for CreateApiKeyRequest {
    fn validate(&self) -> Result<(), CustomError> {
        if self.name.is_empty() {
            return Err(CustomError::ValidationError(
                "API key name should not be empty".into(),
            ));
        }
        if self.name != self.name.trim() {
            return Err(CustomError::ValidationError(
                "API key name must not have leading or trailing whitespace".into(),
            ));
        }
        if self.name.len() > MAX_API_KEY_NAME_LENGTH {
            return Err(CustomError::ValidationError(format!(
                "API key name length should be less than {} bytes",
                MAX_API_KEY_NAME_LENGTH
            )));
        }
        Ok(())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyResponse {
    pub api_key_id: String,
    pub name: String,
    pub key: String,
    pub key_prefix: String,
    pub created_at: i64,
}

#[derive(sqlx::FromRow)]
pub struct ApiKeyListRow {
    pub api_key_id: String,
    pub name: String,
    pub key_prefix: String,
    pub created_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyListItem {
    pub api_key_id: String,
    pub name: String,
    pub key_prefix: String,
    pub created_at: i64,
}

impl From<ApiKeyListRow> for ApiKeyListItem {
    fn from(row: ApiKeyListRow) -> Self {
        Self {
            api_key_id: row.api_key_id,
            name: row.name,
            key_prefix: row.key_prefix,
            created_at: row.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct ApiKeyListResponse {
    pub items: Vec<ApiKeyListItem>,
}
