use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{MessageResponse, ExperimentsDB, EvaluateRequest};
use crate::repository::db_get_experiment_by_id;

pub async fn evaluate(
    db: web::Data<ExperimentsDB>,
    payload: web::Json<EvaluateRequest>,
) -> Result<HttpResponse, CustomError> {
    let request = payload.into_inner();
    request.validate()?;

    let experiment = db_get_experiment_by_id(&db, request.experiment_id).await?;
    if experiment.is_none() {
        return Err(CustomError::NotFoundError(
            format!("Experiment with id '{}' not found", request.experiment_id)
        ));
    }

    // TODO: implement evaluation logic using DuckDB
    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Evaluate endpoint is not implemented yet".to_string(),
    }))
}
