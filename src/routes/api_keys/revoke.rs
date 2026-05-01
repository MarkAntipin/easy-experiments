use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{AuthenticatedUser, ExperimentsDB, MessageResponse};
use crate::services::api_key;

pub async fn revoke_api_key(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    id: web::Path<String>,
) -> Result<HttpResponse, CustomError> {
    api_key::revoke(&db, &id, &user.company_id).await?;
    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "API key revoked".to_string(),
    }))
}
