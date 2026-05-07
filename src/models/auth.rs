use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub company_id: String,
    pub email: String,
}

#[derive(Clone)]
pub struct JwtSecret(pub String);

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct CompanyRow {
    pub company_id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct UserRow {
    pub user_id: String,
    pub company_id: String,
    pub email: String,
    pub name: Option<String>,
    pub picture_url: Option<String>,
    pub google_sub: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize)]
pub struct GoogleLoginRequest {
    pub token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
    pub company: CompanyResponse,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub user_id: String,
    pub email: String,
    pub name: Option<String>,
    pub picture_url: Option<String>,
}

impl From<UserRow> for UserResponse {
    fn from(row: UserRow) -> Self {
        Self {
            user_id: row.user_id,
            email: row.email,
            name: row.name,
            picture_url: row.picture_url,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyResponse {
    pub company_id: String,
    pub name: String,
}

impl From<CompanyRow> for CompanyResponse {
    fn from(row: CompanyRow) -> Self {
        Self {
            company_id: row.company_id,
            name: row.name,
        }
    }
}
