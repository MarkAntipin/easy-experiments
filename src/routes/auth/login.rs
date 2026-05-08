use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{
    CompanyResponse, ExperimentsDB, GoogleLoginRequest, JwtSecret, LoginResponse, UserResponse,
};
use crate::services::auth;
use crate::services::google_auth::GoogleTokenVerifier;

pub async fn google_login(
    db: web::Data<ExperimentsDB>,
    jwt_secret: web::Data<JwtSecret>,
    google_verifier: web::Data<GoogleTokenVerifier>,
    payload: web::Json<GoogleLoginRequest>,
) -> Result<HttpResponse, CustomError> {
    let request = payload.into_inner();
    let result =
        auth::google_login(&db, &google_verifier, &jwt_secret.0, &request.token).await?;

    tracing::info!(
        user_id = %result.user.user_id,
        company_id = %result.company.company_id,
        email = %result.user.email,
        "user logged in via google",
    );

    Ok(HttpResponse::Ok().json(LoginResponse {
        token: result.token,
        user: UserResponse::from(result.user),
        company: CompanyResponse::from(result.company),
    }))
}
