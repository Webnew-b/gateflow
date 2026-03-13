use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataplaneError {
    #[error("app not found")]
    AppNotFound,
    #[error("app disabled")]
    AppDisabled,
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("rate limited")]
    RateLimited,
    #[error("forbidden by ip policy")]
    IpForbidden,
    #[error("proxy error: {0}")]
    Proxy(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for DataplaneError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match &self {
            DataplaneError::AppNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            DataplaneError::AppDisabled => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            DataplaneError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            DataplaneError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            DataplaneError::IpForbidden => (StatusCode::FORBIDDEN, self.to_string()),
            DataplaneError::Proxy(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            DataplaneError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        let body = Json(ErrorBody { error: msg });
        (status, body).into_response()
    }
}
