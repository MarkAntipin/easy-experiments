use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{AuthenticatedUser, ExperimentsDB, InviteUserRequest, UserListItem};
use crate::services::user;
use crate::validation::ValidatedJson;

pub async fn invite_user(
    db: web::Data<ExperimentsDB>,
    actor: web::ReqData<AuthenticatedUser>,
    payload: ValidatedJson<InviteUserRequest>,
) -> Result<HttpResponse, CustomError> {
    let request = payload.into_inner();
    let invited = user::invite(&db, &actor.company_id, &request.email).await?;

    tracing::info!(
        actor_user_id = %actor.user_id,
        company_id = %actor.company_id,
        invited_user_id = %invited.user_id,
        invited_email = %invited.email,
        "user invited",
    );

    Ok(HttpResponse::Created().json(UserListItem::from(invited)))
}
