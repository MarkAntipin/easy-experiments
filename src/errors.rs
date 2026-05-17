use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use sqlx;

#[derive(Debug, Display, Clone)]
pub enum CustomError {
    SerializeError(String),
    ValidationError(String),
    InternalError(String),
    UnauthorizedError(String),
    ForbiddenError(String),
    NotFoundError(String),
    ConflictError(String),
    PreconditionFailedError(String),
}

impl CustomError {
    fn kind(&self) -> &'static str {
        match self {
            CustomError::SerializeError(_) => "SerializeError",
            CustomError::ValidationError(_) => "ValidationError",
            CustomError::InternalError(_) => "InternalError",
            CustomError::UnauthorizedError(_) => "UnauthorizedError",
            CustomError::ForbiddenError(_) => "ForbiddenError",
            CustomError::NotFoundError(_) => "NotFoundError",
            CustomError::ConflictError(_) => "ConflictError",
            CustomError::PreconditionFailedError(_) => "PreconditionFailedError",
        }
    }

    fn message(&self) -> &str {
        match self {
            CustomError::SerializeError(m)
            | CustomError::ValidationError(m)
            | CustomError::InternalError(m)
            | CustomError::UnauthorizedError(m)
            | CustomError::ForbiddenError(m)
            | CustomError::NotFoundError(m)
            | CustomError::ConflictError(m)
            | CustomError::PreconditionFailedError(m) => m,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse<'a> {
    pub message: &'a str,
}

impl ErrorResponse<'_> {
    pub fn to_json(message: &str) -> String {
        let error_response = ErrorResponse { message };
        to_string_pretty(&error_response).unwrap()
    }
}

impl ResponseError for CustomError {
    fn status_code(&self) -> StatusCode {
        match self {
            CustomError::SerializeError(_) => StatusCode::BAD_REQUEST,
            CustomError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            CustomError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CustomError::UnauthorizedError(_) => StatusCode::UNAUTHORIZED,
            CustomError::ForbiddenError(_) => StatusCode::FORBIDDEN,
            CustomError::NotFoundError(_) => StatusCode::NOT_FOUND,
            CustomError::ConflictError(_) => StatusCode::CONFLICT,
            CustomError::PreconditionFailedError(_) => StatusCode::PRECONDITION_FAILED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let message = self.message();
        let kind = self.kind();
        let code = status.as_u16();

        // Centralised: 5xx -> error, 400/401/403 -> warn, other 4xx stay quiet.
        if status.is_server_error() {
            tracing::error!(status = code, kind, error = %message, "request failed");
        } else if matches!(
            status,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::BAD_REQUEST
        ) {
            tracing::warn!(status = code, kind, error = %message, "request rejected");
        }

        HttpResponse::build(status).body(ErrorResponse::to_json(message))
    }
}

impl From<sqlx::Error> for CustomError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!(error = %err, "sqlx error");
        CustomError::InternalError("Internal Server Error".to_string())
    }
}
