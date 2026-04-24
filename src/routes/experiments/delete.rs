use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{AuthenticatedUser, MessageResponse, ExperimentsDB};
use crate::repository::db_delete_experiment;

pub async fn delete_experiment(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    id: web::Path<String>,
) -> Result<HttpResponse, CustomError> {
    let deleted = db_delete_experiment(&db, &id, &user.company_id).await?;

    if !deleted {
        return Err(CustomError::NotFoundError(format!(
            "Experiment with id '{}' not found", id
        )));
    }

    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Experiment deleted".to_string(),
    }))
}
