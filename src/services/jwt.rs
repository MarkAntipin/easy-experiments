use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::errors::CustomError;
use crate::models::AuthenticatedUser;

const JWT_ISSUER: &str = "easy-experiments";
const JWT_AUDIENCE: &str = "easy-experiments-admin";

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    company_id: String,
    email: String,
    iss: String,
    aud: String,
    exp: usize,
}

pub fn create_jwt(user: &AuthenticatedUser, secret: &str) -> Result<String, CustomError> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(7))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user.user_id.clone(),
        company_id: user.company_id.clone(),
        email: user.email.clone(),
        iss: JWT_ISSUER.to_string(),
        aud: JWT_AUDIENCE.to_string(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| CustomError::InternalError(format!("Failed to create JWT: {}", e)))
}

pub fn verify_jwt(token: &str, secret: &str) -> Result<AuthenticatedUser, CustomError> {
    let mut validation = Validation::default();
    validation.set_issuer(&[JWT_ISSUER]);
    validation.set_audience(&[JWT_AUDIENCE]);

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| CustomError::UnauthorizedError(format!("Invalid token: {}", e)))?;

    Ok(AuthenticatedUser {
        user_id: token_data.claims.sub,
        company_id: token_data.claims.company_id,
        email: token_data.claims.email,
    })
}
