use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{
    CompanyResponse, ExperimentsDB, JwtSecret, LoginResponse, PasswordLoginRequest, UserResponse,
};
use crate::services::password_auth;

pub async fn password_login(
    db: web::Data<ExperimentsDB>,
    jwt_secret: web::Data<JwtSecret>,
    payload: web::Json<PasswordLoginRequest>,
) -> Result<HttpResponse, CustomError> {
    let request = payload.into_inner();

    if request.email.trim().is_empty() || request.password.is_empty() {
        return Err(CustomError::UnauthorizedError(
            "invalid email or password".into(),
        ));
    }

    let result =
        password_auth::password_login(&db, &jwt_secret.0, &request.email, &request.password)
            .await?;

    tracing::info!(
        user_id = %result.user.user_id,
        company_id = %result.company.company_id,
        email = %result.user.email,
        "user logged in via password",
    );

    Ok(HttpResponse::Ok().json(LoginResponse {
        token: result.token,
        user: UserResponse::from(result.user),
        company: CompanyResponse::from(result.company),
    }))
}
