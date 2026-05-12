use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{
    AcceptInviteRequest, CompanyResponse, ExperimentsDB, JwtSecret, LoginResponse, UserResponse,
};
use crate::services::password_auth;

pub async fn accept_invite(
    db: web::Data<ExperimentsDB>,
    jwt_secret: web::Data<JwtSecret>,
    payload: web::Json<AcceptInviteRequest>,
) -> Result<HttpResponse, CustomError> {
    let request = payload.into_inner();

    if request.token.trim().is_empty() {
        return Err(CustomError::ValidationError("token is required".into()));
    }

    let result =
        password_auth::accept_invite(&db, &jwt_secret.0, &request.token, &request.password).await?;

    tracing::info!(
        user_id = %result.user.user_id,
        company_id = %result.company.company_id,
        email = %result.user.email,
        "user accepted invite",
    );

    Ok(HttpResponse::Ok().json(LoginResponse {
        token: result.token,
        user: UserResponse::from(result.user),
        company: CompanyResponse::from(result.company),
    }))
}
