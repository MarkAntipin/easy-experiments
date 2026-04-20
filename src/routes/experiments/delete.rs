use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{MessageResponse, ExperimentsDB};
use crate::repository::db_delete_experiment;

pub async fn delete_experiment(
    db: web::Data<ExperimentsDB>,
    id: web::Path<i64>,
) -> Result<HttpResponse, CustomError> {
    let deleted = db_delete_experiment(&db, *id).await?;

    if !deleted {
        return Err(CustomError::NotFoundError(
            format!("Experiment with id '{}' not found", id)
        ));
    }

    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Experiment deleted".to_string(),
    }))
}
