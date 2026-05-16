use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;

#[derive(Debug, Clone, Default)]
pub struct PublicConfig {
    pub google_client_id: Option<String>,
}

#[derive(Serialize)]
struct AppConfigPayload<'a> {
    #[serde(rename = "googleClientId")]
    google_client_id: &'a str,
}

pub async fn config_js(cfg: web::Data<PublicConfig>) -> impl Responder {
    let google = cfg
        .google_client_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("");
    let payload = AppConfigPayload {
        google_client_id: google,
    };
    // serde_json escapes the value so an unusual client id (with quotes,
    // backslashes, etc.) can't break out of the JS literal.
    let json = serde_json::to_string(&payload).expect("PublicConfig is always serializable");
    let body = format!("window.__APP_CONFIG = {};\n", json);
    HttpResponse::Ok()
        .content_type("application/javascript; charset=utf-8")
        // no-store so a `GOOGLE_CLIENT_ID` change at restart takes effect on
        // the next page load instead of being masked by a stale cache.
        .insert_header(("Cache-Control", "no-store"))
        .body(body)
}
