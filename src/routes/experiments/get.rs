use actix_web::web;
use actix_web::HttpResponse;
use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::{
    AuthenticatedUser, ExperimentListItem, ExperimentResponse, ExperimentsDB,
    GetExperimentsQueryParams,
};
use crate::services::experiment;

pub async fn get_experiment_by_id(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse, CustomError> {
    let id = id.into_inner().to_string();
    let row = experiment::get_experiment(&db, &id, &user.company_id).await?;
    Ok(HttpResponse::Ok().json(ExperimentResponse::from_row(row)?))
}

pub async fn get_experiments(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    query: web::Query<GetExperimentsQueryParams>,
) -> Result<HttpResponse, CustomError> {
    let rows =
        experiment::list_experiments(&db, query.status.clone(), &user.company_id).await?;
    let items: Vec<ExperimentListItem> = rows.into_iter().map(Into::into).collect();
    Ok(HttpResponse::Ok().json(items))
}
