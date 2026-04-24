use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{AuthenticatedUser, ExperimentsDB, MessageResponse};
use crate::repository::db_revoke_api_key;

pub async fn revoke_api_key(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    id: web::Path<String>,
) -> Result<HttpResponse, CustomError> {
    let revoked = db_revoke_api_key(&db, &id, &user.company_id).await?;

    if !revoked {
        return Err(CustomError::NotFoundError(format!(
            "API key with id '{}' not found", id
        )));
    }

    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "API key revoked".to_string(),
    }))
}
