use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{AuthenticatedUser, ExperimentsDB, MessageResponse};
use crate::services::experiment;

pub async fn delete_experiment(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    id: web::Path<String>,
) -> Result<HttpResponse, CustomError> {
    experiment::delete_experiment(&db, &id, &user.company_id).await?;
    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Experiment deleted".to_string(),
    }))
}
