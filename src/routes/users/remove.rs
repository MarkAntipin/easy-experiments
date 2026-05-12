use actix_web::web;
use actix_web::HttpResponse;
use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::{AuthenticatedUser, ExperimentsDB};
use crate::services::user;

pub async fn remove_user(
    db: web::Data<ExperimentsDB>,
    actor: web::ReqData<AuthenticatedUser>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse, CustomError> {
    let id = id.into_inner().to_string();
    user::remove(&db, &id, &actor.company_id, &actor.user_id).await?;

    tracing::info!(
        actor_user_id = %actor.user_id,
        company_id = %actor.company_id,
        removed_user_id = %id,
        "user removed",
    );

    Ok(HttpResponse::NoContent().finish())
}
