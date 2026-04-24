use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{ApiKeyListItem, AuthenticatedUser, ExperimentsDB};
use crate::repository::db_list_api_keys;

pub async fn list_api_keys(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
) -> Result<HttpResponse, CustomError> {
    let rows = db_list_api_keys(&db, &user.company_id).await?;
    let items: Vec<ApiKeyListItem> = rows.into_iter().map(Into::into).collect();
    Ok(HttpResponse::Ok().json(items))
}
