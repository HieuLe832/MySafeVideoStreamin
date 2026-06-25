use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::presentation::web::router::PresentationState;
use crate::presentation::web::handlers::{ApiError, ApiResult};

#[derive(Deserialize)]
pub struct UploadFromUrlRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct UploadFromUrlResponse {
    pub key: String,
}

// Handler lấy danh sách tác vụ tải ngầm đang hoạt động hoặc đã bị lỗi
pub async fn list_downloads_handler(
    State(state): State<Arc<PresentationState>>,
) -> impl IntoResponse {
    let downloads = state.list_downloads_use_case.execute();
    Json(downloads)
}

// Handler xóa tác vụ lỗi khỏi danh sách quản lý
pub async fn dismiss_download_handler(
    State(state): State<Arc<PresentationState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if state.dismiss_download_use_case.execute(&id) {
        state.upload_from_url_use_case.process_queue();
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

// Handler tải video từ URL và upload trực tiếp lên R2 chạy ngầm
pub async fn upload_from_url_handler(
    State(state): State<Arc<PresentationState>>,
    Json(payload): Json<UploadFromUrlRequest>,
) -> ApiResult<impl IntoResponse> {
    let task_id = state.upload_from_url_use_case.execute(&payload.url).await
        .map_err(ApiError::BadRequest)?;

    Ok((StatusCode::ACCEPTED, Json(UploadFromUrlResponse { key: task_id })))
}
