use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{ApiKeyListItem, AuthenticatedUser, ExperimentsDB};
use crate::services::api_key;

pub async fn list_api_keys(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
) -> Result<HttpResponse, CustomError> {
    let rows = api_key::list(&db, &user.company_id).await?;
    let items: Vec<ApiKeyListItem> = rows.into_iter().map(Into::into).collect();
    Ok(HttpResponse::Ok().json(items))
}
