use axum::{
    routing::{delete, get, post},
    Router,
    middleware as axum_middleware,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use crate::presentation::web::handlers::{
    auth::auth_config_handler,
    video::{list_videos_handler, create_upload_url_handler, get_stream_url_handler, delete_video_handler, rename_video_handler, get_thumbnail_handler},
    download::{list_downloads_handler, dismiss_download_handler, upload_from_url_handler},
};
use crate::presentation::web::middleware::{
    auth::auth_middleware,
    security::security_headers_middleware,
};
use crate::application::use_cases::{
    auth::VerifyTokenUseCase,
    video::{ListVideosUseCase, GetUploadUrlUseCase, GetStreamUrlUseCase, DeleteVideoUseCase, RenameVideoUseCase},
    download::{ListDownloadsUseCase, DismissDownloadUseCase, UploadFromUrlUseCase},
    thumbnail::GetThumbnailUseCase,
};

pub struct PresentationState {
    pub verify_token_use_case: Arc<VerifyTokenUseCase>,
    pub list_videos_use_case: Arc<ListVideosUseCase>,
    pub get_upload_url_use_case: Arc<GetUploadUrlUseCase>,
    pub get_stream_url_use_case: Arc<GetStreamUrlUseCase>,
    pub delete_video_use_case: Arc<DeleteVideoUseCase>,
    pub rename_video_use_case: Arc<RenameVideoUseCase>,
    pub list_downloads_use_case: Arc<ListDownloadsUseCase>,
    pub dismiss_download_use_case: Arc<DismissDownloadUseCase>,
    pub upload_from_url_use_case: Arc<UploadFromUrlUseCase>,
    pub get_thumbnail_use_case: Arc<GetThumbnailUseCase>,
    pub google_client_id: String,
    pub csp_header_value: String,
}

pub fn create_router(
    state: Arc<PresentationState>,
    allowed_origin_str: &str,
) -> Router {
    // Cấu hình CORS để hỗ trợ gọi API từ Vercel Frontend
    let mut cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any);

    if allowed_origin_str == "*" {
        cors = cors.allow_origin(Any);
    } else {
        let origins: Vec<axum::http::HeaderValue> = allowed_origin_str
            .split(',')
            .map(|s| s.trim().parse::<axum::http::HeaderValue>().expect("ALLOWED_ORIGIN must be a valid header value"))
            .collect();
        cors = cors.allow_origin(origins);
    }

    // Cấu hình API Routes
    // Các endpoint videos yêu cầu bảo mật qua middleware
    let api_routes = Router::new()
        .route("/videos", get(list_videos_handler))
        .route("/videos/upload-url", post(create_upload_url_handler))
        .route("/videos/upload-from-url", post(upload_from_url_handler))
        .route("/videos/downloads", get(list_downloads_handler))
        .route("/videos/downloads/:id", delete(dismiss_download_handler))
        .route("/videos/:key/stream-url", get(get_stream_url_handler))
        .route("/videos/:key/thumbnail", get(get_thumbnail_handler))
        .route("/videos/:key", delete(delete_video_handler))
        .route("/videos/:key/rename", post(rename_video_handler))
        .route_layer(axum_middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state.clone());

    // Endpoint công khai để lấy cấu hình Auth
    let auth_routes = Router::new()
        .route("/config", get(auth_config_handler))
        .with_state(state.clone());

    // Phục vụ Frontend Static Files
    // Khi chạy production, backend sẽ phục vụ thư mục frontend/dist được build sẵn
    let frontend_service = ServeDir::new("./frontend/dist")
        .not_found_service(ServeFile::new("./frontend/dist/index.html"));

    Router::new()
        .nest("/api", api_routes)
        .nest("/api/auth", auth_routes)
        .route("/health", get(health_check_handler).head(health_check_handler))
        .fallback_service(frontend_service)
        .layer(axum_middleware::from_fn_with_state(state.clone(), security_headers_middleware))
        .layer(cors)
}

// Handler kiểm tra trạng thái hoạt động (Health check cho Uptime Robot)
async fn health_check_handler() -> axum::http::StatusCode {
    axum::http::StatusCode::OK
}
