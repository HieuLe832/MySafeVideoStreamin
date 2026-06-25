use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub enum ApiError {
    Internal(String),
    BadRequest(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match self {
            ApiError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
            ApiError::BadRequest(e) => (StatusCode::BAD_REQUEST, e),
        };
        (status, Json(ErrorResponse { error: msg })).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

pub mod auth;
pub mod video;
pub mod download;
