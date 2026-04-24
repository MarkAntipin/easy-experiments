use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{
    AuthenticatedUser, ExperimentListItem, ExperimentResponse, ExperimentStatus, ExperimentsDB,
    GetExperimentsQueryParams,
};
use crate::repository::{db_get_experiment_by_id, db_get_experiments};

pub async fn get_experiment_by_id(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    id: web::Path<String>,
) -> Result<HttpResponse, CustomError> {
    let result = db_get_experiment_by_id(&db, &id, &user.company_id).await?;

    match result {
        Some(experiment) => {
            Ok(HttpResponse::Ok().json(ExperimentResponse::from_row(experiment)?))
        }
        None => Err(CustomError::NotFoundError(format!(
            "Experiment with id '{}' not found", id
        ))),
    }
}

pub async fn get_experiments(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    query: web::Query<GetExperimentsQueryParams>,
) -> Result<HttpResponse, CustomError> {
    if let Some(ExperimentStatus::Deleted) = query.status {
        return Err(CustomError::ValidationError(
            "Filtering by 'deleted' status is not allowed".to_string(),
        ));
    }

    let experiments = db_get_experiments(&db, query.status.clone(), &user.company_id).await?;
    let items: Vec<ExperimentListItem> = experiments.into_iter().map(Into::into).collect();
    Ok(HttpResponse::Ok().json(items))
}
