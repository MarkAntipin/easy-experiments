use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub company_id: String,
    pub email: String,
}

#[derive(Clone, Debug)]
pub struct AuthenticatedApiKey {
    pub api_key_id: String,
    pub company_id: String,
}

#[derive(Clone)]
pub struct JwtSecret(pub String);
