use actix_web::web;
use actix_web::HttpResponse;
use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::{AuthenticatedUser, ExperimentsDB};
use crate::services::experiment;

pub async fn delete_experiment(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse, CustomError> {
    let id = id.into_inner().to_string();
    experiment::delete_experiment(&db, &id, &user.company_id).await?;

    tracing::info!(
        actor_user_id = %user.user_id,
        company_id = %user.company_id,
        experiment_id = %id,
        "experiment deleted",
    );

    Ok(HttpResponse::NoContent().finish())
}
