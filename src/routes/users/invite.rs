use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{
    AuthenticatedUser, ExperimentsDB, InviteConfig, InviteUserRequest, InviteUserResponse,
    UserListItem,
};
use crate::services::user;
use crate::validation::ValidatedJson;

pub async fn invite_user(
    db: web::Data<ExperimentsDB>,
    invite_cfg: web::Data<InviteConfig>,
    password_enabled: web::Data<PasswordAuthEnabled>,
    actor: web::ReqData<AuthenticatedUser>,
    payload: ValidatedJson<InviteUserRequest>,
) -> Result<HttpResponse, CustomError> {
    if !actor.role.is_admin() {
        return Err(CustomError::ForbiddenError(
            "Only admins can invite members".into(),
        ));
    }
    let request = payload.into_inner();

    let ttl = if password_enabled.0 {
        Some(invite_cfg.token_ttl_days)
    } else {
        None
    };

    let result = user::invite(&db, &actor.company_id, &request.email, ttl).await?;

    tracing::info!(
        actor_user_id = %actor.user_id,
        company_id = %actor.company_id,
        invited_user_id = %result.user.user_id,
        invited_email = %result.user.email,
        password_invite = result.plaintext_token.is_some(),
        "user invited",
    );

    let expires_at = result.user.invite_token_expires_at;
    let invite_url = result
        .plaintext_token
        .as_ref()
        .and_then(|tok| build_invite_url(&invite_cfg.app_base_url, tok));

    let response = InviteUserResponse {
        user: UserListItem::from(result.user),
        invite_token: result.plaintext_token,
        invite_url,
        invite_expires_at: expires_at,
    };

    Ok(HttpResponse::Created().json(response))
}

/// Whether the password provider is enabled in this deployment. Wrapped in a
/// newtype so it can be registered with `web::Data` distinct from other bools.
pub struct PasswordAuthEnabled(pub bool);

fn build_invite_url(base: &str, token: &str) -> Option<String> {
    let trimmed = base.trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    Some(format!(
        "{trimmed}/accept-invite?token={}",
        urlencoding(token)
    ))
}

/// Token is base64-url-no-pad so the only chars are `A-Z a-z 0-9 - _` — none of
/// which need URL-encoding. Still, a defensive pass costs nothing.
fn urlencoding(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            other => format!("%{:02X}", other as u32),
        })
        .collect()
}
