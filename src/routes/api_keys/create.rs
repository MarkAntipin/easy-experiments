use actix_web::web;
use actix_web::HttpResponse;

use crate::errors::CustomError;
use crate::models::{
    AuthenticatedUser, CreateApiKeyRequest, CreateApiKeyResponse, ExperimentsDB,
};
use crate::repository::{db_create_api_key, NewApiKey};
use crate::services::api_key::generate_api_key;
use crate::validation::ValidatedJson;

pub async fn create_api_key(
    db: web::Data<ExperimentsDB>,
    user: web::ReqData<AuthenticatedUser>,
    payload: ValidatedJson<CreateApiKeyRequest>,
) -> Result<HttpResponse, CustomError> {
    let request = payload.into_inner();
    let generated = generate_api_key();

    let (api_key_id, created_at) = db_create_api_key(
        &db,
        NewApiKey {
            company_id: &user.company_id,
            name: &request.name,
            key_hash: &generated.hash,
            key_prefix: &generated.prefix,
        },
    )
    .await?;

    Ok(HttpResponse::Created().json(CreateApiKeyResponse {
        api_key_id,
        name: request.name,
        key: generated.plaintext,
        key_prefix: generated.prefix,
        created_at,
    }))
}
