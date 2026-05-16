use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;

use actix_cors::Cors;
use actix_files::{Files, NamedFile};
use actix_web::{
    body::MessageBody,
    dev::{fn_service, Server, ServiceRequest, ServiceResponse},
    error::{JsonPayloadError, PathError},
    middleware::{from_fn, Next},
    web, App, Error, HttpMessage, HttpRequest, HttpServer, Result,
};
use tracing_actix_web::TracingLogger;

const EVALUATE_JSON_BODY_LIMIT: usize = 32 * 1024;

const TRACK_JSON_BODY_LIMIT: usize = 256 * 1024;

const ADMIN_JSON_BODY_LIMIT: usize = 256 * 1024;

use crate::{
    config::AuthProviders,
    errors::CustomError,
    models::{ExperimentsDB, InviteConfig, JwtSecret},
    repository::{db_fetch_user_role, db_find_api_key_by_hash},
    routes::{
        accept_invite, config_js, create_api_key, create_experiment, delete_experiment, evaluate,
        get_experiment_by_id, get_experiment_results, get_experiments, google_login, health_check,
        invite_user, list_api_keys, list_users, password_login, remove_user, revoke_api_key,
        start_experiment, stop_experiment, track, update_experiment, PasswordAuthEnabled,
        PublicConfig,
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
            return Err(CustomError::UnauthorizedError(
                "missing or invalid Authorization header".to_string(),
            )
            .into());
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
    google_verifier: Option<GoogleTokenVerifier>,
    auth_providers: AuthProviders,
    invite_config: InviteConfig,
    cors_allowed_origins: Vec<String>,
    event_sink: Arc<dyn EventSink>,
    metric_sink: Arc<dyn MetricSink>,
    results_service: Arc<ResultsService>,
    ui_dist_path: Option<PathBuf>,
    public_config: PublicConfig,
) -> Result<Server, std::io::Error> {
    // Defensive: callers derive the mode from config; this guards against
    // accidentally passing an empty `AuthProviders` from some new caller.
    if !auth_providers.google && !auth_providers.password {
        panic!("at least one auth provider must be enabled — nobody could sign in otherwise");
    }

    let ui_dist_path = ui_dist_path.filter(|p| p.is_dir());
    if let Some(ref p) = ui_dist_path {
        tracing::info!(path = %p.display(), "serving UI bundle on /");
    }

    let db = web::Data::new(db);
    let jwt_secret = web::Data::new(JwtSecret(jwt_secret));
    let google_verifier = google_verifier.map(web::Data::new);
    let invite_config = web::Data::new(invite_config);
    let password_enabled = web::Data::new(PasswordAuthEnabled(auth_providers.password));
    let public_config = web::Data::new(public_config);
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

        let auth_scope = {
            let mut scope = web::scope("/auth");
            if auth_providers.google {
                scope = scope.route("/google", web::post().to(google_login));
            }
            if auth_providers.password {
                scope = scope
                    .route("/login", web::post().to(password_login))
                    .route("/accept-invite", web::post().to(accept_invite));
            }
            scope
        };

        let app = App::new()
            .wrap(cors)
            .wrap(TracingLogger::default())
            .service(
                web::scope("/admin/v1")
                    .app_data(
                        web::JsonConfig::default()
                            .limit(ADMIN_JSON_BODY_LIMIT)
                            .error_handler(json_validation_error),
                    )
                    .app_data(web::PathConfig::default().error_handler(path_validation_error))
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
                            .route("/{id}/results", web::get().to(get_experiment_results)),
                    )
                    .service(
                        web::scope("/api-keys")
                            .wrap(from_fn(jwt_auth_middleware))
                            .route("", web::post().to(create_api_key))
                            .route("", web::get().to(list_api_keys))
                            .route("/{id}", web::delete().to(revoke_api_key)),
                    )
                    .service(
                        web::scope("/users")
                            .wrap(from_fn(jwt_auth_middleware))
                            .route("", web::post().to(invite_user))
                            .route("", web::get().to(list_users))
                            .route("/{id}", web::delete().to(remove_user)),
                    )
                    .service(auth_scope),
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
                            .route("/evaluate", web::post().to(evaluate)),
                    )
                    .service(
                        web::scope("/track")
                            .app_data(
                                web::JsonConfig::default()
                                    .limit(TRACK_JSON_BODY_LIMIT)
                                    .error_handler(json_validation_error),
                            )
                            .wrap(from_fn(api_key_auth_middleware))
                            .route("", web::post().to(track)),
                    ),
            )
            .route("/health", web::get().to(health_check))
            .route("/config.js", web::get().to(config_js))
            .app_data(db.clone())
            .app_data(jwt_secret.clone())
            .app_data(invite_config.clone())
            .app_data(password_enabled.clone())
            .app_data(public_config.clone())
            .app_data(event_sink.clone())
            .app_data(metric_sink.clone())
            .app_data(results_service.clone())
            .configure(json_error_handler);

        let app = match google_verifier.clone() {
            Some(v) => app.app_data(v),
            None => app,
        };

        // Static UI must be registered AFTER /admin/v1, /api/v1, /health so
        // API routes win when paths overlap. Unknown paths fall back to
        // index.html
        match ui_dist_path.clone() {
            Some(dist) => {
                let index = dist.join("index.html");
                app.service(
                    Files::new("/", dist)
                        .index_file("index.html")
                        .default_handler(fn_service(move |req: ServiceRequest| {
                            let index = index.clone();
                            async move {
                                let (req, _) = req.into_parts();
                                let file = NamedFile::open_async(&index).await?;
                                let res = file.into_response(&req);
                                Ok(ServiceResponse::new(req, res))
                            }
                        })),
                )
            }
            None => app,
        }
    })
    .listen(listener)?
    .run();
    Ok(server)
}
