use actix_web::web;
use actix_web::HttpResponse;
use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::{AuthenticatedUser, ExperimentsDB, GetResultsQueryParams};
use crate::services::analytics::ResultsService;

pub async fn get_experiment_results(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    service: web::Data<ResultsService>,
    id: web::Path<Uuid>,
    query: web::Query<GetResultsQueryParams>,
) -> Result<HttpResponse, CustomError> {
    let id = id.into_inner().to_string();
    let params = query.into_inner();
    params.validate()?;

    let response = service
        .get_results(
            db.get_ref(),
            &user.company_id,
            &id,
            crate::services::analytics::results::ResultsParams {
                start_ms: params.start,
                end_ms: params.end,
                granularity: params.granularity,
                metric_name: params.metric,
            },
        )
        .await?;

    Ok(HttpResponse::Ok().json(response.as_ref()))
}
