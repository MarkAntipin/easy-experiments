use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub company_id: String,
    pub email: String,
}

#[derive(Clone)]
pub struct JwtSecret(pub String);
