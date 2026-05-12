use std::net::TcpListener;
use std::sync::Arc;

use actix_web::{
    Result,
    body::MessageBody,
    dev::{Server, ServiceRequest, ServiceResponse},
    error::{JsonPayloadError, PathError},
    middleware::{from_fn, Next},
    web, App, Error, HttpRequest, HttpServer, HttpMessage,
};
use actix_cors::Cors;
use tracing_actix_web::TracingLogger;

/// Body limit for `/evaluate`. The valid request shape is `experiment_key` +
/// `entity_id` (each ≤256 bytes) + a `properties` JSON object. 32KB leaves
/// generous room for properties while bounding the worst case at the edge,
/// so the in-validate "is this too big?" walk can stay out of the hot path.
const EVALUATE_JSON_BODY_LIMIT: usize = 32 * 1024;

/// Body limit for `/track`. Up to 100 events per request × per-event ceiling
/// (entity_id 256B + metric_name 256B + idempotency 128B + value/ts overhead).
/// 256KB leaves headroom for batched SDK flushes.
const TRACK_JSON_BODY_LIMIT: usize = 256 * 1024;

/// Body limit for admin write endpoints. Sized so a maxed-out
/// `CreateExperimentRequest` (per the validator caps in `models/request.rs`)
/// still fits, while keeping per-request RAM bounded on a small VPS.
const ADMIN_JSON_BODY_LIMIT: usize = 256 * 1024;

use crate::{
    errors::CustomError,
    models::{ExperimentsDB, JwtSecret},
    repository::{db_fetch_user_role, db_find_api_key_by_hash},
    routes::{
        create_api_key,
        create_experiment,
        delete_experiment,
        evaluate,
        get_experiment_by_id,
        get_experiment_results,
        get_experiments,
        google_login,
        health_check,
        invite_user,
        list_api_keys,
        list_users,
        remove_user,
        revoke_api_key,
        start_experiment,
        stop_experiment,
        track,
        update_experiment,
    },
    services::analytics::ResultsService,
    services::api_key::{hash_api_key, is_plausible_api_key},
    services::exposure::EventSink,
    services::google_auth::GoogleTokenVerifier,
    services::jwt::verify_jwt,
    services::metric_sink::MetricSink,
};

fn json_validation_error(err: JsonPayloadError, _req: &HttpRequest) -> Error {
    let err_message = err.to_string();
    let clean_error_message = err_message
        .split(" at line")
        .next()
        .map(str::to_string)
        .unwrap_or(err_message);
    CustomError::ValidationError(clean_error_message).into()
}

fn path_validation_error(err: PathError, _req: &HttpRequest) -> Error {
    CustomError::ValidationError(format!("Invalid path parameter: {}", err)).into()
}

pub fn json_error_handler(cfg: &mut web::ServiceConfig) {
    cfg.app_data(web::JsonConfig::default().error_handler(json_validation_error));
}

async fn api_key_auth_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let provided = req
        .headers()
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            CustomError::UnauthorizedError("missing or invalid `X-Api-Key` header".to_string())
        })?;

    if !is_plausible_api_key(&provided) {
        return Err(CustomError::UnauthorizedError("invalid API key".to_string()).into());
    }

    let db = req
        .app_data::<web::Data<ExperimentsDB>>()
        .ok_or_else(|| CustomError::InternalError("Database not configured".to_string()))?;

    let hash = hash_api_key(&provided);
    let row = db_find_api_key_by_hash(db, &hash)
        .await?
        .ok_or_else(|| CustomError::UnauthorizedError("invalid API key".to_string()))?;

    let authenticated = crate::models::AuthenticatedApiKey {
        api_key_id: row.api_key_id.clone(),
        company_id: row.company_id.clone(),
    };

    req.extensions_mut().insert(authenticated);

    next.call(req).await
}

async fn jwt_auth_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let token = match auth_header {
        Some(ref header) if header.starts_with("Bearer ") => &header[7..],
        _ => {
            return Err(
                CustomError::UnauthorizedError("missing or invalid Authorization header".to_string())
                    .into(),
            );
        }
    };

    let jwt_secret = req
        .app_data::<web::Data<JwtSecret>>()
        .ok_or_else(|| CustomError::InternalError("JWT secret not configured".to_string()))?;

    let identity = verify_jwt(token, &jwt_secret.0)?;

    // Signature alone isn't enough: a JWT minted before the user was removed
    // would still be valid. Re-read the row on every request so a deleted
    // user's open browser window stops working immediately, and so role
    // changes take effect without a token refresh.
    let db = req
        .app_data::<web::Data<ExperimentsDB>>()
        .ok_or_else(|| CustomError::InternalError("Database not configured".to_string()))?;
    let role = db_fetch_user_role(db, &identity.user_id, &identity.company_id)
        .await?
        .ok_or_else(|| CustomError::UnauthorizedError("user no longer has access".to_string()))?;

    let user = crate::models::AuthenticatedUser {
        user_id: identity.user_id,
        company_id: identity.company_id,
        email: identity.email,
        role,
    };

    req.extensions_mut().insert(user);

    next.call(req).await
}

pub fn run(
    listener: TcpListener,
    db: ExperimentsDB,
    jwt_secret: String,
    google_verifier: GoogleTokenVerifier,
    cors_allowed_origins: Vec<String>,
    event_sink: Arc<dyn EventSink>,
    metric_sink: Arc<dyn MetricSink>,
    results_service: Arc<ResultsService>,
) -> Result<Server, std::io::Error> {
    let db = web::Data::new(db);
    let jwt_secret = web::Data::new(JwtSecret(jwt_secret));
    let google_verifier = web::Data::new(google_verifier);
    let event_sink: web::Data<dyn EventSink> = web::Data::from(event_sink);
    let metric_sink: web::Data<dyn MetricSink> = web::Data::from(metric_sink);
    let results_service = web::Data::from(results_service);

    let server = HttpServer::new(move || {
        let mut cors = Cors::default()
            .allowed_methods(vec!["GET", "POST", "PATCH", "DELETE", "OPTIONS"])
            .max_age(3600);
        for origin in &cors_allowed_origins {
            cors = cors.allowed_origin(origin);
        }

        App::new()
            .wrap(cors)
            .wrap(TracingLogger::default())
            .service(
                web::scope("/admin/v1")
                .app_data(
                    web::JsonConfig::default()
                        .limit(ADMIN_JSON_BODY_LIMIT)
                        .error_handler(json_validation_error),
                )
                .app_data(
                    web::PathConfig::default().error_handler(path_validation_error),
                )
                .service(
                    web::scope("/experiments")
                        .wrap(from_fn(jwt_auth_middleware))
                        .route("", web::post().to(create_experiment))
                        .route("", web::get().to(get_experiments))
                        .route("/{id}", web::get().to(get_experiment_by_id))
                        .route("/{id}", web::patch().to(update_experiment))
                        .route("/{id}", web::delete().to(delete_experiment))
                        .route("/{id}/start", web::post().to(start_experiment))
                        .route("/{id}/stop", web::post().to(stop_experiment))
                        .route("/{id}/results", web::get().to(get_experiment_results))
                )
                .service(
                    web::scope("/api-keys")
                        .wrap(from_fn(jwt_auth_middleware))
                        .route("", web::post().to(create_api_key))
                        .route("", web::get().to(list_api_keys))
                        .route("/{id}", web::delete().to(revoke_api_key))
                )
                .service(
                    web::scope("/users")
                        .wrap(from_fn(jwt_auth_middleware))
                        .route("", web::post().to(invite_user))
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::delete().to(remove_user))
                )
                .service(
                    web::scope("/auth")
                        .route("/google", web::post().to(google_login))
                )
            )
            .service(
                web::scope("/api/v1")
                .service(
                    web::scope("/experiments")
                        .app_data(
                            web::JsonConfig::default()
                                .limit(EVALUATE_JSON_BODY_LIMIT)
                                .error_handler(json_validation_error),
                        )
                        .wrap(from_fn(api_key_auth_middleware))
                        .route("/evaluate", web::post().to(evaluate))
                )
                .service(
                    web::scope("/track")
                        .app_data(
                            web::JsonConfig::default()
                                .limit(TRACK_JSON_BODY_LIMIT)
                                .error_handler(json_validation_error),
                        )
                        .wrap(from_fn(api_key_auth_middleware))
                        .route("", web::post().to(track))
                )
            )
            .route("/health", web::get().to(health_check))
            .app_data(db.clone())
            .app_data(jwt_secret.clone())
            .app_data(google_verifier.clone())
            .app_data(event_sink.clone())
            .app_data(metric_sink.clone())
            .app_data(results_service.clone())
            .configure(json_error_handler)
    })
    .listen(listener)?
    .run();
    Ok(server)
}
