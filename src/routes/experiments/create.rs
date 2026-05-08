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

    tracing::info!(
        actor_user_id = %user.user_id,
        company_id = %user.company_id,
        experiment_id = %id,
        "experiment created",
    );

    Ok(HttpResponse::Created().json(CreateExperimentResponse {
        experiment_id: id,
    }))
}
