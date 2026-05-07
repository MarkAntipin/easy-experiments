use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{ApiKeyListResponse, AuthenticatedUser, ExperimentsDB};
use crate::services::api_key;

pub async fn list_api_keys(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
) -> Result<HttpResponse, CustomError> {
    let items = api_key::list(&db, &user.company_id).await?;
    Ok(HttpResponse::Ok().json(ApiKeyListResponse { items }))
}
