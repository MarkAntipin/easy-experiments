use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{MessageResponse, CreateExperimentRequest, ExperimentDBRow, ExperimentsDB};
use crate::repository::db_create_experiment;

pub async fn create_experiment(
    db: web::Data<ExperimentsDB>,
    payload: web::Json<CreateExperimentRequest>,
) -> Result<HttpResponse, CustomError> {
    let request = payload.into_inner();
    request.validate()?;

    let row: ExperimentDBRow = request.into();
    let id = db_create_experiment(&db, &row).await?;

    if id.is_none() {
        return Err(CustomError::ConflictError(
            format!("Experiment with name '{}' already exists", row.name)
        ));
    }

    let response = MessageResponse {
        message: format!("Experiment '{}' created with id {}", row.name, id.unwrap()),
    };
    Ok(HttpResponse::Created().json(response))
}
