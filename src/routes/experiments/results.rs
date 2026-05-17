use actix_web::web;
use actix_web::HttpResponse;
use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::{AuthenticatedUser, ExperimentsDB};
use crate::services::analytics::ResultsService;

pub async fn get_experiment_results(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    service: web::Data<ResultsService>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse, CustomError> {
    let id = id.into_inner().to_string();
    let response = service
        .get_results(db.get_ref(), &user.company_id, &id)
        .await?;
    Ok(HttpResponse::Ok().json(response.as_ref()))
}
