use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::enums::ExperimentStatus;
use crate::models::{MessageResponse, ExperimentsDB, UpdateExperimentRequest};
use crate::repository::{db_update_experiment, db_update_experiment_status};

pub async fn update_experiment(
    db: web::Data<ExperimentsDB>,
    id: web::Path<i64>,
    payload: web::Json<UpdateExperimentRequest>,
) -> Result<HttpResponse, CustomError> {
    let request = payload.into_inner();
    request.validate()?;

    let variants_json = request.variants.as_ref().map(|v| {
        serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string())
    });

    let result = db_update_experiment(
        &db,
        *id,
        request.name.as_deref(),
        request.description.as_deref(),
        variants_json.as_deref(),
        request.traffic_percentage,
    ).await?;

    if result.is_none() {
        return Err(CustomError::NotFoundError(
            format!("Experiment with id '{}' not found", id)
        ));
    }

    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Experiment updated".to_string(),
    }))
}

pub async fn enable_experiment(
    db: web::Data<ExperimentsDB>,
    id: web::Path<i64>,
) -> Result<HttpResponse, CustomError> {
    let result = db_update_experiment_status(&db, *id, ExperimentStatus::Active).await?;

    if result.is_none() {
        return Err(CustomError::NotFoundError(
            format!("Experiment with id '{}' not found", id)
        ));
    }

    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Experiment enabled".to_string(),
    }))
}

pub async fn disable_experiment(
    db: web::Data<ExperimentsDB>,
    id: web::Path<i64>,
) -> Result<HttpResponse, CustomError> {
    let result = db_update_experiment_status(&db, *id, ExperimentStatus::Paused).await?;

    if result.is_none() {
        return Err(CustomError::NotFoundError(
            format!("Experiment with id '{}' not found", id)
        ));
    }

    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Experiment disabled".to_string(),
    }))
}
