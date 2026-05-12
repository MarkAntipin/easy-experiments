use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{AuthenticatedUser, ExperimentsDB, UserListResponse};
use crate::services::user;

pub async fn list_users(
    db: web::Data<ExperimentsDB>,
    actor: web::ReqData<AuthenticatedUser>,
) -> Result<HttpResponse, CustomError> {
    let items = user::list(&db, &actor.company_id).await?;
    Ok(HttpResponse::Ok().json(UserListResponse { items }))
}
