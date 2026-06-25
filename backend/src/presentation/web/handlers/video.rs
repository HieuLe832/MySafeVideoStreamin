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
pub struct ListVideosParams {
    pub refresh: Option<bool>,
}

// Handler liệt kê video
pub async fn list_videos_handler(
    State(state): State<Arc<PresentationState>>,
    axum::extract::Query(params): axum::extract::Query<ListVideosParams>,
) -> ApiResult<impl IntoResponse> {
    let force_refresh = params.refresh.unwrap_or(false);
    let (videos, total_used_bytes, max_limit_bytes) = state.list_videos_use_case.execute(force_refresh).await
        .map_err(ApiError::Internal)?;

    #[derive(Serialize)]
    struct ListResponse {
        videos: Vec<crate::domain::models::VideoInfo>,
        total_used_bytes: u64,
        max_limit_bytes: u64,
    }

    Ok(Json(ListResponse {
        videos,
        total_used_bytes,
        max_limit_bytes,
    }))
}

#[derive(Deserialize)]
pub struct UploadUrlRequest {
    pub file_name: String,
    pub file_size: u64,
}

#[derive(Serialize)]
pub struct UploadUrlResponse {
    pub upload_url: String,
    pub key: String,
}

// Handler tạo presigned upload URL
pub async fn create_upload_url_handler(
    State(state): State<Arc<PresentationState>>,
    Json(payload): Json<UploadUrlRequest>,
) -> ApiResult<impl IntoResponse> {
    if payload.file_name.trim().is_empty() {
        return Err(ApiError::BadRequest("Tên file không được trống.".into()));
    }

    let (upload_url, key) = state.get_upload_url_use_case.execute(&payload.file_name, payload.file_size).await
        .map_err(ApiError::BadRequest)?;

    Ok((StatusCode::CREATED, Json(UploadUrlResponse { upload_url, key })))
}

#[derive(Serialize)]
pub struct StreamUrlResponse {
    pub stream_url: String,
}

// Handler lấy presigned stream URL
pub async fn get_stream_url_handler(
    State(state): State<Arc<PresentationState>>,
    Path(key): Path<String>,
) -> ApiResult<impl IntoResponse> {
    if key.trim().is_empty() {
        return Err(ApiError::BadRequest("Key không hợp lệ.".into()));
    }

    let stream_url = state.get_stream_url_use_case.execute(&key).await
        .map_err(ApiError::Internal)?;

    Ok(Json(StreamUrlResponse { stream_url }))
}

#[derive(Deserialize)]
pub struct ThumbnailParams {
    pub _token: Option<String>,
    pub debug: Option<bool>,
}

// Handler lấy ảnh thumbnail (nếu chưa có thì tạo lazy-load bằng ffmpeg)
pub async fn get_thumbnail_handler(
    State(state): State<Arc<PresentationState>>,
    Path(key): Path<String>,
    axum::extract::Query(params): axum::extract::Query<ThumbnailParams>,
) -> impl IntoResponse {
    if key.trim().is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    match state.get_thumbnail_use_case.execute(&key).await {
        Ok(thumb_url) => {
            axum::response::Redirect::temporary(&thumb_url).into_response()
        }
        Err(e) => {
            tracing::warn!("Failed to get or generate thumbnail for key {}: {}. Returning 404.", key, e);
            if params.debug.unwrap_or(false) {
                return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response();
            }
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

// Handler xóa video
pub async fn delete_video_handler(
    State(state): State<Arc<PresentationState>>,
    Path(key): Path<String>,
) -> ApiResult<impl IntoResponse> {
    if key.trim().is_empty() {
        return Err(ApiError::BadRequest("Key không hợp lệ.".into()));
    }

    state.delete_video_use_case.execute(&key).await
        .map_err(ApiError::Internal)?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct RenameVideoRequest {
    pub new_name: String,
}

// Handler đổi tên hiển thị video
pub async fn rename_video_handler(
    State(state): State<Arc<PresentationState>>,
    Path(key): Path<String>,
    Json(payload): Json<RenameVideoRequest>,
) -> ApiResult<impl IntoResponse> {
    if key.trim().is_empty() {
        return Err(ApiError::BadRequest("Key không hợp lệ.".into()));
    }
    if payload.new_name.trim().is_empty() {
        return Err(ApiError::BadRequest("Tên file mới không được để trống.".into()));
    }

    state.rename_video_use_case.execute(&key, &payload.new_name).await
        .map_err(ApiError::Internal)?;

    Ok(StatusCode::OK)
}
