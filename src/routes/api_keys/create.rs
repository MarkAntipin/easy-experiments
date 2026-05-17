use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{AuthenticatedUser, CreateApiKeyRequest, CreateApiKeyResponse, ExperimentsDB};
use crate::services::api_key;
use crate::validation::ValidatedJson;

pub async fn create_api_key(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    payload: ValidatedJson<CreateApiKeyRequest>,
) -> Result<HttpResponse, CustomError> {
    let request = payload.into_inner();
    let created = api_key::create(&db, &user.company_id, request.name).await?;

    tracing::info!(
        actor_user_id = %user.user_id,
        company_id = %user.company_id,
        api_key_id = %created.api_key_id,
        key_prefix = %created.prefix,
        "api key created",
    );

    Ok(HttpResponse::Created().json(CreateApiKeyResponse {
        api_key_id: created.api_key_id,
        name: created.name,
        key: created.plaintext,
        key_prefix: created.prefix,
        created_at: created.created_at,
    }))
}
