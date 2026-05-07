use crate::errors::CustomError;
use crate::models::{
    validate_experiment_state, validate_segments_compatible_for_running, CreateExperimentRequest,
    ExperimentListRow, ExperimentRow, ExperimentStatus, ExperimentsDB, Segment,
    UpdateExperimentRequest, Variant,
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
) -> Result<Vec<ExperimentListRow>, CustomError> {
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

    if let Some(expected) = if_match {
        if expected != existing.updated_at {
            return Err(CustomError::PreconditionFailedError(
                "Experiment was modified by another request; refetch and retry".into(),
            ));
        }
    }

    let existing_variants: Vec<Variant> = serde_json::from_str(&existing.variants).map_err(|e| {
        CustomError::InternalError(format!("Failed to parse stored variants: {}", e))
    })?;
    let existing_segments: Vec<Segment> = serde_json::from_str(&existing.segments).map_err(|e| {
        CustomError::InternalError(format!("Failed to parse stored segments: {}", e))
    })?;

    let description_change: Option<Option<String>> = match request.description {
        None => None,
        Some(new_desc) => {
            let same = match (&new_desc, &existing.description) {
                (Some(a), Some(b)) => a == b,
                (None, None) => true,
                _ => false,
            };
            if same { None } else { Some(new_desc) }
        }
    };

    let primary_metric_change: Option<String> = match request.primary_metric {
        None => None,
        Some(new_pm) if new_pm == existing.primary_metric => None,
        Some(new_pm) => {
            if existing.status != ExperimentStatus::Draft {
                return Err(CustomError::ConflictError(format!(
                    "Cannot change primaryMetric of experiment in '{}' status",
                    existing.status
                )));
            }
            Some(new_pm)
        }
    };

    let variants_change: Option<Vec<Variant>> = match request.variants {
        None => None,
        Some(new_variants) if new_variants == existing_variants => None,
        Some(new_variants) => {
            if existing.status != ExperimentStatus::Draft {
                return Err(CustomError::ConflictError(format!(
                    "Cannot change variants of experiment in '{}' status",
                    existing.status
                )));
            }
            Some(new_variants)
        }
    };

    let segments_change: Option<Vec<Segment>> = match request.segments {
        None => None,
        Some(new_segments) if new_segments == existing_segments => None,
        Some(new_segments) => match existing.status {
            ExperimentStatus::Draft => Some(new_segments),
            ExperimentStatus::Running => {
                validate_segments_compatible_for_running(&existing_segments, &new_segments)?;
                Some(new_segments)
            }
            ref other => {
                return Err(CustomError::ConflictError(format!(
                    "Cannot change segments of experiment in '{}' status",
                    other
                )));
            }
        },
    };

    if description_change.is_none()
        && primary_metric_change.is_none()
        && variants_change.is_none()
        && segments_change.is_none()
    {
        // Nothing actually changed. Verifying the row + version (above) is
        // enough; skip the DB write so we don't bump updated_at on a no-op.
        return Ok(());
    }

    let effective_description: Option<&str> = match &description_change {
        Some(new) => new.as_deref(),
        None => existing.description.as_deref(),
    };
    let effective_primary_metric: &str = primary_metric_change
        .as_deref()
        .unwrap_or(&existing.primary_metric);
    let effective_variants: &[Variant] = variants_change
        .as_deref()
        .unwrap_or(&existing_variants);
    let effective_segments: &[Segment] = segments_change
        .as_deref()
        .unwrap_or(&existing_segments);

    validate_experiment_state(
        effective_description,
        effective_primary_metric,
        effective_variants,
        effective_segments,
    )?;

    let fields = UpdateExperimentFields {
        description: description_change.as_ref().map(|inner| inner.as_deref()),
        primary_metric: primary_metric_change.as_deref(),
        variants: variants_change.as_deref(),
        segments: segments_change.as_deref(),
    };

    // Always pass the loaded updated_at so the write loses the WHERE if the
    // row was mutated (including a status transition) since we read it.
    match db_update_experiment(db, id, company_id, fields, Some(existing.updated_at)).await? {
        UpdateExperimentOutcome::Updated => Ok(()),
        UpdateExperimentOutcome::NotFound => Err(CustomError::NotFoundError(format!(
            "Experiment with id '{}' not found",
            id
        ))),
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
