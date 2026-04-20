use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{ExperimentsDB, GetExperimentsQueryParams};
use crate::repository::{db_get_experiments, db_get_experiment_by_id};

pub async fn get_experiment_by_id(
    db: web::Data<ExperimentsDB>,
    id: web::Path<i64>,
) -> Result<HttpResponse, CustomError> {
    let experiment = db_get_experiment_by_id(&db, *id).await?;
    if experiment.is_none() {
        return Err(CustomError::NotFoundError(format!("Experiment with id '{}' not found", id)));
    }
    Ok(HttpResponse::Ok().json(experiment))
}

pub async fn get_experiments(
    db: web::Data<ExperimentsDB>,
    query: web::Query<GetExperimentsQueryParams>,
) -> Result<HttpResponse, CustomError> {
    let experiments = db_get_experiments(&db, query.status.clone()).await?;
    Ok(HttpResponse::Ok().json(experiments))
}
