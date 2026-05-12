use crate::errors::CustomError;
use crate::models::{AuthenticatedUser, CompanyRow, ExperimentsDB, UserRow};
use crate::repository::{
    db_bind_user_google_sub, db_create_user_and_company, db_find_user_by_email,
    db_find_user_by_google_sub, db_update_user_profile,
};
use crate::services::funny_names::generate_funny_name;
use crate::services::google_auth::{GoogleTokenInfo, GoogleTokenVerifier};
use crate::services::jwt::create_jwt;

pub struct GoogleLoginResult {
    pub token: String,
    pub user: UserRow,
    pub company: CompanyRow,
}

pub async fn google_login(
    db: &ExperimentsDB,
    verifier: &GoogleTokenVerifier,
    jwt_secret: &str,
    google_token: &str,
) -> Result<GoogleLoginResult, CustomError> {
    let token_info = verifier.verify(google_token).await?;
    provision_and_mint(db, &token_info, jwt_secret).await
}

/// Resolve (and possibly create) the user identified by a verified Google
/// token, then mint a JWT. Split out from `google_login` so tests can exercise
/// the provisioning rules without a real Google token.
pub async fn provision_and_mint(
    db: &ExperimentsDB,
    token_info: &GoogleTokenInfo,
    jwt_secret: &str,
) -> Result<GoogleLoginResult, CustomError> {
    let token_name = Some(token_info.name.as_str()).filter(|s| !s.is_empty());
    let token_picture = Some(token_info.picture.as_str()).filter(|s| !s.is_empty());
    let normalized_email = token_info.email.trim().to_lowercase();

    let (mut user, company) = resolve_user(db, token_info, &normalized_email).await?;

    if user.name.as_deref() != token_name || user.picture_url.as_deref() != token_picture {
        db_update_user_profile(db, &user.user_id, token_name, token_picture).await?;
        user.name = token_name.map(str::to_string);
        user.picture_url = token_picture.map(str::to_string);
    }

    let auth_user = AuthenticatedUser {
        user_id: user.user_id.clone(),
        company_id: user.company_id.clone(),
        email: user.email.clone(),
    };

    let token = create_jwt(&auth_user, jwt_secret)?;

    Ok(GoogleLoginResult {
        token,
        user,
        company,
    })
}

async fn resolve_user(
    db: &ExperimentsDB,
    token_info: &GoogleTokenInfo,
    normalized_email: &str,
) -> Result<(UserRow, CompanyRow), CustomError> {
    if let Some(pair) = db_find_user_by_google_sub(db, &token_info.sub).await? {
        return Ok(pair);
    }

    match db_find_user_by_email(db, normalized_email).await? {
        Some((stub, company)) if stub.google_sub.is_none() => {
            // Claim a pending invite: bind the real google_sub onto the stub.
            db_bind_user_google_sub(db, &stub.user_id, &token_info.sub).await?;
            let claimed = UserRow {
                google_sub: Some(token_info.sub.clone()),
                ..stub
            };
            Ok((claimed, company))
        }
        Some(_) => {
            // Email is taken by an already-activated user whose google_sub
            // does not match this token's sub. Refuse rather than corrupt the
            // existing identity binding.
            Err(CustomError::ConflictError(
                "An account with this email already exists. Sign in with the original Google account."
                    .into(),
            ))
        }
        None => {
            let token_name = Some(token_info.name.as_str()).filter(|s| !s.is_empty());
            let token_picture = Some(token_info.picture.as_str()).filter(|s| !s.is_empty());
            db_create_user_and_company(
                db,
                normalized_email,
                &generate_funny_name(),
                token_name,
                token_picture,
                &token_info.sub,
            )
            .await
        }
    }
}
