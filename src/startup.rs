use std::net::TcpListener;

use actix_web::{
    Result,
    body::MessageBody,
    dev::{Server, ServiceRequest, ServiceResponse},
    middleware::{from_fn, Next, Logger},
    web, App, Error, HttpServer, HttpMessage,
};
use actix_cors::Cors;

use crate::{
    errors::CustomError,
    models::{ExperimentsDB, JwtSecret},
    routes::{
        create_experiment,
        get_experiments,
        get_experiment_by_id,
        update_experiment,
        start_experiment,
        stop_experiment,
        delete_experiment,
        evaluate,
        health_check,
        google_login,
    },
    services::google_auth::GoogleTokenVerifier,
    services::jwt::verify_jwt,
};

pub fn json_error_handler(cfg: &mut web::ServiceConfig) {
    cfg.app_data(web::JsonConfig::default().error_handler(|err, _req| {
        let err_message = err.to_string();

        let clean_error_message = match err_message.split(" at line").next() {
            Some(msg) => msg.to_string(),
            None => err_message,
        };

        CustomError::ValidationError(clean_error_message.to_string()).into()
    }));
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

    let expected = req
        .app_data::<web::Data<String>>()
        .ok_or_else(|| CustomError::InternalError("API key not configured".to_string()))?;

    if provided.as_str() != expected.get_ref().as_str() {
        return Err(CustomError::ForbiddenError("invalid `X-Api-Key` header".to_string()).into());
    }

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

    let user = verify_jwt(token, &jwt_secret.0)?;

    req.extensions_mut().insert(user);

    next.call(req).await
}

pub fn run(
    listener: TcpListener,
    db: ExperimentsDB,
    api_key: String,
    jwt_secret: String,
    google_verifier: GoogleTokenVerifier,
    cors_allowed_origins: Vec<String>,
) -> Result<Server, std::io::Error> {
    let db = web::Data::new(db);
    let api_key = web::Data::new(api_key);
    let jwt_secret = web::Data::new(JwtSecret(jwt_secret));
    let google_verifier = web::Data::new(google_verifier);

    let server = HttpServer::new(move || {
        let mut cors = Cors::default()
            .allowed_methods(vec!["GET", "POST", "PATCH", "DELETE", "OPTIONS"])
            .allowed_headers(vec!["Authorization", "Content-Type", "X-Api-Key"])
            .max_age(3600);
        for origin in &cors_allowed_origins {
            cors = cors.allowed_origin(origin);
        }

        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .service(
                web::scope("/admin/v1")
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
                        .wrap(from_fn(api_key_auth_middleware))
                        .route("/evaluate", web::post().to(evaluate))
                )
            )
            .route("/health", web::get().to(health_check))
            .app_data(api_key.clone())
            .app_data(db.clone())
            .app_data(jwt_secret.clone())
            .app_data(google_verifier.clone())
            .configure(json_error_handler)
    })
    .listen(listener)?
    .run();
    Ok(server)
}
