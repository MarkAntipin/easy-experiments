use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{
    AuthenticatedUser, CompanyResponse, ExperimentsDB, GoogleLoginRequest, JwtSecret,
    LoginResponse, UserResponse,
};
use crate::repository::{
    db_create_user_and_company, db_find_user_by_google_sub, db_update_user_profile,
};
use crate::services::google_auth::GoogleTokenVerifier;
use crate::services::jwt::create_jwt;

pub async fn google_login(
    db: web::Data<ExperimentsDB>,
    jwt_secret: web::Data<JwtSecret>,
    google_verifier: web::Data<GoogleTokenVerifier>,
    payload: web::Json<GoogleLoginRequest>,
) -> Result<HttpResponse, CustomError> {
    let request = payload.into_inner();

    let token_info = google_verifier.verify(&request.token).await?;

    let token_name = Some(token_info.name.as_str()).filter(|s| !s.is_empty());
    let token_picture = Some(token_info.picture.as_str()).filter(|s| !s.is_empty());

    let (mut user, company) = match db_find_user_by_google_sub(&db, &token_info.sub).await? {
        Some((u, c)) => (u, c),
        None => {
            db_create_user_and_company(
                &db,
                &token_info.email,
                token_name,
                token_picture,
                &token_info.sub,
            )
            .await?
        }
    };

    if user.name.as_deref() != token_name || user.picture_url.as_deref() != token_picture {
        db_update_user_profile(&db, &user.user_id, token_name, token_picture).await?;
        user.name = token_name.map(str::to_string);
        user.picture_url = token_picture.map(str::to_string);
    }

    let auth_user = AuthenticatedUser {
        user_id: user.user_id.clone(),
        company_id: user.company_id.clone(),
        email: user.email.clone(),
    };

    let token = create_jwt(&auth_user, &jwt_secret.0)?;

    Ok(HttpResponse::Ok().json(LoginResponse {
        token,
        user: UserResponse {
            user_id: user.user_id,
            email: user.email,
            name: user.name,
            picture_url: user.picture_url,
        },
        company: CompanyResponse {
            company_id: company.company_id,
            name: company.name,
        },
    }))
}
