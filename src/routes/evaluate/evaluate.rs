use actix_web::web;
use actix_web::HttpResponse;

use crate::analytics::EventSink;
use crate::errors::CustomError;
use crate::models::{AuthenticatedApiKey, EvaluateRequest, EvaluateResponse, ExperimentsDB};
use crate::services::evaluation;
use crate::validation::ValidatedJson;

pub async fn evaluate(
    db: web::Data<ExperimentsDB>,
    api_key: web::ReqData<AuthenticatedApiKey>,
    sink: web::Data<dyn EventSink>,
    payload: ValidatedJson<EvaluateRequest>,
) -> Result<HttpResponse, CustomError> {
    let result = evaluation::evaluate(
        &db,
        sink.get_ref(),
        &api_key.company_id,
        payload.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(EvaluateResponse {
        experiment_key: result.experiment_key,
        variant_key: result.variant_key,
        config: result.config,
    }))
}
