use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{
    AuthenticatedUser, CreateExperimentRequest, CreateExperimentResponse, ExperimentsDB,
};
use crate::repository::db_create_experiment;
use crate::validation::ValidatedJson;

pub async fn create_experiment(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    payload: ValidatedJson<CreateExperimentRequest>,
) -> Result<HttpResponse, CustomError> {
    let request = payload.into_inner();

    let id = db_create_experiment(
        &db,
        &request.key,
        request.description.as_deref(),
        &request.primary_metric,
        &request.variants,
        &request.segments,
        &user.company_id,
    )
    .await?;

    Ok(HttpResponse::Created().json(CreateExperimentResponse {
        experiment_id: id,
        message: "Experiment created".to_string(),
    }))
}
