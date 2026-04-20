use std::net::TcpListener;

use actix_web::{
    Result,
    body::MessageBody,
    dev::{Server, ServiceRequest, ServiceResponse},
    middleware::{from_fn, Next, Logger},
    web, App, Error, HttpServer
};
use actix_cors::Cors;

use crate::{
    errors::CustomError,
    models::ExperimentsDB,
    routes::{
        create_experiment,
        get_experiments,
        get_experiment_by_id,
        update_experiment,
        enable_experiment,
        disable_experiment,
        delete_experiment,
        evaluate,
        health_check,
        validate_token,
    },
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

async fn auth_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let headers = req.headers();
    let api_key_header = headers.get("X-Api-Key");

    if api_key_header.is_none() {
        return Err(CustomError::UnauthorizedError("missing `X-Api-Key` header".to_string()).into());
    }

    let expected_api_key = req.app_data::<web::Data<String>>().unwrap();
    if api_key_header.unwrap().to_str().unwrap() != expected_api_key.to_string() {
        return Err(CustomError::ForbiddenError("invalid `X-Api-Key` header".to_string()).into());
    }
    next.call(req).await
}

pub fn run(
    listener: TcpListener,
    db: ExperimentsDB,
    api_key: String,
) -> Result<Server, std::io::Error> {
    let db = web::Data::new(db);
    let api_key = web::Data::new(api_key);

    let server = HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .service(
                web::scope("/api/v1")
                .service(
                    web::scope("/experiments")
                        .wrap(from_fn(auth_middleware))
                        .route("", web::post().to(create_experiment))
                        .route("", web::get().to(get_experiments))
                        .route("/{id}", web::get().to(get_experiment_by_id))
                        .route("/{id}", web::put().to(update_experiment))
                        .route("/{id}", web::delete().to(delete_experiment))
                        .route("/{id}/enable", web::put().to(enable_experiment))
                        .route("/{id}/disable", web::put().to(disable_experiment))
                )
                .service(
                    web::scope("/evaluate")
                        .wrap(from_fn(auth_middleware))
                        .route("", web::post().to(evaluate))
                )
                .service(
                    web::scope("/auth")
                        .route("/validate-token", web::post().to(validate_token))
                )
            )
            .route("/health", web::get().to(health_check))
            .app_data(api_key.clone())
            .app_data(db.clone())
            .configure(json_error_handler)
    })
    .listen(listener)?
    .run();
    Ok(server)
}
