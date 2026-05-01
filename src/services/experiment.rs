use crate::errors::CustomError;
use crate::models::{
    validate_experiment_state, CreateExperimentRequest, ExperimentRow, ExperimentStatus,
    ExperimentsDB, Segment, UpdateExperimentRequest, Variant,
};
use crate::repository::{
    db_create_experiment, db_delete_experiment, db_get_experiment_by_id, db_get_experiments,
    db_start_experiment, db_stop_experiment, db_update_experiment, CreateExperimentOutcome,
    DeleteExperimentOutcome, StartExperimentOutcome, StopExperimentOutcome,
    UpdateExperimentFields, UpdateExperimentOutcome,
};

pub async fn create_experiment(
    db: &ExperimentsDB,
    company_id: &str,
    request: CreateExperimentRequest,
) -> Result<String, CustomError> {
    match db_create_experiment(
        db,
        &request.key,
        request.description.as_deref(),
        &request.primary_metric,
        &request.variants,
        &request.segments,
        company_id,
    )
    .await?
    {
        CreateExperimentOutcome::Created(id) => Ok(id),
        CreateExperimentOutcome::KeyConflict => Err(CustomError::ConflictError(format!(
            "Experiment with key '{}' already exists",
            request.key
        ))),
    }
}

pub async fn get_experiment(
    db: &ExperimentsDB,
    id: &str,
    company_id: &str,
) -> Result<ExperimentRow, CustomError> {
    db_get_experiment_by_id(db, id, company_id)
        .await?
        .ok_or_else(|| {
            CustomError::NotFoundError(format!("Experiment with id '{}' not found", id))
        })
}

pub async fn list_experiments(
    db: &ExperimentsDB,
    status: Option<ExperimentStatus>,
    company_id: &str,
) -> Result<Vec<ExperimentRow>, CustomError> {
    if let Some(ExperimentStatus::Deleted) = status {
        return Err(CustomError::ValidationError(
            "Filtering by 'deleted' status is not allowed".to_string(),
        ));
    }
    db_get_experiments(db, status, company_id).await
}

pub async fn update_experiment(
    db: &ExperimentsDB,
    id: &str,
    company_id: &str,
    request: UpdateExperimentRequest,
    if_match: Option<i64>,
) -> Result<(), CustomError> {
    let existing = get_experiment(db, id, company_id).await?;

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

    match db_update_experiment(db, id, company_id, fields, if_match).await? {
        UpdateExperimentOutcome::Updated => Ok(()),
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
    db: &ExperimentsDB,
    id: &str,
    company_id: &str,
) -> Result<(), CustomError> {
    match db_start_experiment(db, id, company_id).await? {
        StartExperimentOutcome::Started => Ok(()),
        StartExperimentOutcome::NotFound => Err(CustomError::NotFoundError(format!(
            "Experiment with id '{}' not found",
            id
        ))),
        StartExperimentOutcome::NotInDraft(status) => Err(CustomError::ConflictError(format!(
            "Can only start experiments in 'draft' status, current status is '{}'",
            status
        ))),
    }
}

pub async fn stop_experiment(
    db: &ExperimentsDB,
    id: &str,
    company_id: &str,
) -> Result<(), CustomError> {
    match db_stop_experiment(db, id, company_id).await? {
        StopExperimentOutcome::Stopped => Ok(()),
        StopExperimentOutcome::NotFound => Err(CustomError::NotFoundError(format!(
            "Experiment with id '{}' not found",
            id
        ))),
        StopExperimentOutcome::NotRunning(status) => Err(CustomError::ConflictError(format!(
            "Can only stop experiments in 'running' status, current status is '{}'",
            status
        ))),
    }
}

pub async fn delete_experiment(
    db: &ExperimentsDB,
    id: &str,
    company_id: &str,
) -> Result<(), CustomError> {
    match db_delete_experiment(db, id, company_id).await? {
        DeleteExperimentOutcome::Deleted => Ok(()),
        DeleteExperimentOutcome::NotFound => Err(CustomError::NotFoundError(format!(
            "Experiment with id '{}' not found",
            id
        ))),
        DeleteExperimentOutcome::NotDeletable(status) => Err(CustomError::ConflictError(format!(
            "Cannot delete experiment in '{}' status, stop it first",
            status
        ))),
    }
}
