use actix_web::{web, HttpRequest, HttpResponse};
use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::{
    AuthenticatedUser, ExperimentsDB, MessageResponse, UpdateExperimentRequest,
};
use crate::services::experiment;
use crate::validation::ValidatedJson;

fn parse_if_match(req: &HttpRequest) -> Result<Option<i64>, CustomError> {
    let header = match req.headers().get("If-Match") {
        Some(h) => h,
        None => return Ok(None),
    };
    let raw = header
        .to_str()
        .map_err(|_| CustomError::ValidationError("Invalid If-Match header".into()))?;
    let trimmed = raw.trim().trim_matches('"');
    let value = trimmed.parse::<i64>().map_err(|_| {
        CustomError::ValidationError("If-Match must be the experiment's updatedAt timestamp".into())
    })?;
    Ok(Some(value))
}

pub async fn update_experiment(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    id: web::Path<Uuid>,
    req: HttpRequest,
    payload: ValidatedJson<UpdateExperimentRequest>,
) -> Result<HttpResponse, CustomError> {
    let id = id.into_inner().to_string();
    let if_match = parse_if_match(&req)?;
    experiment::update_experiment(&db, &id, &user.company_id, payload.into_inner(), if_match)
        .await?;

    tracing::info!(
        actor_user_id = %user.user_id,
        company_id = %user.company_id,
        experiment_id = %id,
        "experiment updated",
    );

    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Experiment updated".to_string(),
    }))
}

pub async fn start_experiment(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse, CustomError> {
    let id = id.into_inner().to_string();
    experiment::start_experiment(&db, &id, &user.company_id).await?;

    tracing::info!(
        actor_user_id = %user.user_id,
        company_id = %user.company_id,
        experiment_id = %id,
        "experiment started",
    );

    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Experiment started".to_string(),
    }))
}

pub async fn stop_experiment(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse, CustomError> {
    let id = id.into_inner().to_string();
    experiment::stop_experiment(&db, &id, &user.company_id).await?;

    tracing::info!(
        actor_user_id = %user.user_id,
        company_id = %user.company_id,
        experiment_id = %id,
        "experiment stopped",
    );

    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Experiment stopped".to_string(),
    }))
}
