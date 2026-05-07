use actix_web::web;
use actix_web::HttpResponse;
use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::{AuthenticatedUser, ExperimentsDB};
use crate::services::api_key;

pub async fn revoke_api_key(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse, CustomError> {
    let id = id.into_inner().to_string();
    api_key::revoke(&db, &id, &user.company_id).await?;
    Ok(HttpResponse::NoContent().finish())
}
