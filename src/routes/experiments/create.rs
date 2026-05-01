use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{
    AuthenticatedUser, CreateExperimentRequest, CreateExperimentResponse, ExperimentsDB,
};
use crate::services::experiment;
use crate::validation::ValidatedJson;

pub async fn create_experiment(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    payload: ValidatedJson<CreateExperimentRequest>,
) -> Result<HttpResponse, CustomError> {
    let id = experiment::create_experiment(&db, &user.company_id, payload.into_inner()).await?;

    Ok(HttpResponse::Created().json(CreateExperimentResponse {
        experiment_id: id,
        message: "Experiment created".to_string(),
    }))
}
