use actix_web::{web, HttpRequest, HttpResponse};

use crate::errors::CustomError;
use crate::models::{
    validate_experiment_state, AuthenticatedUser, ExperimentsDB, MessageResponse, Segment,
    UpdateExperimentRequest, Variant,
};
use crate::repository::{
    db_get_experiment_by_id, db_start_experiment, db_stop_experiment, db_update_experiment,
    UpdateExperimentFields, UpdateExperimentOutcome,
};
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
    id: web::Path<String>,
    req: HttpRequest,
    payload: ValidatedJson<UpdateExperimentRequest>,
) -> Result<HttpResponse, CustomError> {
    let request = payload.into_inner();
    let if_match = parse_if_match(&req)?;

    let existing = db_get_experiment_by_id(&db, &id, &user.company_id)
        .await
        .map_err(CustomError::from)?
        .ok_or_else(|| {
            CustomError::NotFoundError(format!("Experiment with id '{}' not found", id))
        })?;

    let new_description_ref: Option<Option<&str>> = request
        .description
        .as_ref()
        .map(|inner| inner.as_deref());

    let effective_description: Option<&str> = match &request.description {
        Some(inner) => inner.as_deref(),
        None => existing.description.as_deref(),
    };
    let effective_primary_metric: &str = request
        .primary_metric
        .as_deref()
        .unwrap_or(&existing.primary_metric);

    let existing_variants: Option<Vec<Variant>> = if request.variants.is_none() {
        Some(serde_json::from_str(&existing.variants).map_err(|e| {
            CustomError::InternalError(format!("Failed to parse stored variants: {}", e))
        })?)
    } else {
        None
    };
    let existing_segments: Option<Vec<Segment>> = if request.segments.is_none() {
        Some(serde_json::from_str(&existing.segments).map_err(|e| {
            CustomError::InternalError(format!("Failed to parse stored segments: {}", e))
        })?)
    } else {
        None
    };

    let effective_variants: &[Variant] = request
        .variants
        .as_deref()
        .unwrap_or_else(|| existing_variants.as_deref().unwrap());
    let effective_segments: &[Segment] = request
        .segments
        .as_deref()
        .unwrap_or_else(|| existing_segments.as_deref().unwrap());

    validate_experiment_state(
        effective_description,
        effective_primary_metric,
        effective_variants,
        effective_segments,
    )?;

    let fields = UpdateExperimentFields {
        description: new_description_ref,
        primary_metric: request.primary_metric.as_deref(),
        variants: request.variants.as_deref(),
        segments: request.segments.as_deref(),
    };

    let outcome =
        db_update_experiment(&db, &id, &user.company_id, fields, if_match).await?;

    match outcome {
        UpdateExperimentOutcome::Updated => Ok(HttpResponse::Ok().json(MessageResponse {
            message: "Experiment updated".to_string(),
        })),
        UpdateExperimentOutcome::NotFound => Err(CustomError::NotFoundError(format!(
            "Experiment with id '{}' not found",
            id
        ))),
        UpdateExperimentOutcome::StatusConflict(current_status) => {
            Err(CustomError::ConflictError(format!(
                "Cannot modify primary_metric, variants, or segments of experiment in '{}' status",
                current_status
            )))
        }
        UpdateExperimentOutcome::VersionConflict => Err(CustomError::PreconditionFailedError(
            "Experiment was modified by another request; refetch and retry".into(),
        )),
    }
}

pub async fn start_experiment(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    id: web::Path<String>,
) -> Result<HttpResponse, CustomError> {
    let result = db_start_experiment(&db, &id, &user.company_id).await?;

    match result {
        Some(_) => Ok(HttpResponse::Ok().json(MessageResponse {
            message: "Experiment started".to_string(),
        })),
        None => Err(CustomError::NotFoundError(format!(
            "Experiment with id '{}' not found",
            id
        ))),
    }
}

pub async fn stop_experiment(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    id: web::Path<String>,
) -> Result<HttpResponse, CustomError> {
    let result = db_stop_experiment(&db, &id, &user.company_id).await?;

    match result {
        Some(_) => Ok(HttpResponse::Ok().json(MessageResponse {
            message: "Experiment stopped".to_string(),
        })),
        None => Err(CustomError::NotFoundError(format!(
            "Experiment with id '{}' not found",
            id
        ))),
    }
}
