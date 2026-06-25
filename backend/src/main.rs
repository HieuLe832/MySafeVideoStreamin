use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod r2;
mod domain;
mod application;
mod infrastructure;
mod presentation;
mod utils;

#[cfg(test)]
mod tests;

use crate::r2::R2Service;
use crate::infrastructure::config::InfrastructureConfig;
use crate::infrastructure::google_auth_verifier::GoogleAuthVerifier;
use crate::infrastructure::memory_download_repository::InMemoryDownloadRepository;
use crate::application::use_cases::{
    auth::VerifyTokenUseCase,
    video::{ListVideosUseCase, GetUploadUrlUseCase, GetStreamUrlUseCase, DeleteVideoUseCase, RenameVideoUseCase},
    download::{ListDownloadsUseCase, DismissDownloadUseCase, UploadFromUrlUseCase},
    thumbnail::GetThumbnailUseCase,
};
use crate::presentation::web::router::{create_router, PresentationState};

#[tokio::main]
async fn main() {
    // 1. Khởi tạo Logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "video-streaming-backend=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. Load cấu hình & Adapters
    let config = InfrastructureConfig::load();

    let http_client = reqwest::Client::new();
    let download_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(3600)) // 60 minutes timeout for large downloads
        .no_gzip()
        .no_brotli()
        .build()
        .expect("Failed to build secure HTTP client for download");

    let google_auth_verifier = Arc::new(GoogleAuthVerifier::new(
        config.google_client_id.clone(),
        http_client,
    ));

    let video_repository = Arc::new(R2Service::new(
        &config.r2_account_id,
        &config.r2_access_key_id,
        &config.r2_secret_access_key,
        &config.r2_bucket_name,
    ));

    let download_repository = Arc::new(InMemoryDownloadRepository::new());

    // 3. Khởi tạo Use Cases (Dependency Injection)
    let verify_token_use_case = Arc::new(VerifyTokenUseCase::new(
        google_auth_verifier,
        config.allowed_email.clone(),
    ));

    let list_videos_use_case = Arc::new(ListVideosUseCase::new(video_repository.clone()));
    let get_upload_url_use_case = Arc::new(GetUploadUrlUseCase::new(video_repository.clone()));
    let get_stream_url_use_case = Arc::new(GetStreamUrlUseCase::new(video_repository.clone()));
    let delete_video_use_case = Arc::new(DeleteVideoUseCase::new(video_repository.clone()));
    let rename_video_use_case = Arc::new(RenameVideoUseCase::new(video_repository.clone()));

    let list_downloads_use_case = Arc::new(ListDownloadsUseCase::new(download_repository.clone()));
    let dismiss_download_use_case = Arc::new(DismissDownloadUseCase::new(download_repository.clone()));
    let upload_from_url_use_case = Arc::new(UploadFromUrlUseCase::new(
        download_repository,
        video_repository.clone(),
        download_client,
    ));
    let get_thumbnail_use_case = Arc::new(GetThumbnailUseCase::new(video_repository));

    // 4. Khởi tạo Presentation State
    let presentation_state = Arc::new(PresentationState {
        verify_token_use_case,
        list_videos_use_case,
        get_upload_url_use_case,
        get_stream_url_use_case,
        delete_video_use_case,
        rename_video_use_case,
        list_downloads_use_case,
        dismiss_download_use_case,
        upload_from_url_use_case,
        get_thumbnail_use_case,
        google_client_id: config.google_client_id.clone(),
        csp_header_value: config.csp_header_value.clone(),
    });

    // 5. Cấu hình router & chạy server
    let app = create_router(presentation_state, &config.allowed_origin_str);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("Server is running on http://localhost:{}", config.port);
    axum::serve(listener, app).await.unwrap();
}
