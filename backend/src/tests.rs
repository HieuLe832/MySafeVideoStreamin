mod utils_tests;
mod auth_use_case_tests;
mod download_use_case_tests;
mod security_middleware_tests;
mod auth_middleware_tests;

use std::sync::Arc;
use crate::presentation::web::router::PresentationState;
use crate::application::use_cases::{
    auth::VerifyTokenUseCase,
    video::{ListVideosUseCase, GetUploadUrlUseCase, GetStreamUrlUseCase, DeleteVideoUseCase, RenameVideoUseCase},
    download::{ListDownloadsUseCase, DismissDownloadUseCase, UploadFromUrlUseCase},
    thumbnail::GetThumbnailUseCase,
};
use crate::infrastructure::memory_download_repository::InMemoryDownloadRepository;
use crate::domain::ports::auth_verifier::AuthVerifier;
use crate::domain::ports::video_repository::VideoRepository;
use crate::domain::models::VideoInfo;

struct MockAuth;
#[axum::async_trait]
impl AuthVerifier for MockAuth {
    async fn verify_token(&self, token: &str) -> Result<String, String> {
        if token == "invalid-token" {
            Err("Mã xác thực Google không hợp lệ hoặc đã hết hạn.".to_string())
        } else {
            Ok("admin@example.com".to_string())
        }
    }
}

struct MockVideo;
#[axum::async_trait]
impl VideoRepository for MockVideo {
    async fn get_total_storage_size(&self) -> Result<u64, String> { Ok(0) }
    async fn list_videos(&self, _force_refresh: bool) -> Result<Vec<VideoInfo>, String> { Ok(vec![]) }
    async fn generate_upload_url(&self, _name: &str, _sz: u64) -> Result<(String, String), String> { Ok(("".into(), "".into())) }
    async fn generate_stream_url(&self, _key: &str) -> Result<String, String> { Ok("".into()) }
    async fn upload_file_from_path(&self, _p: &std::path::Path, _name: &str) -> Result<String, String> { Ok("".into()) }
    async fn delete_video(&self, _key: &str) -> Result<(), String> { Ok(()) }
    async fn rename_video(&self, _key: &str, _new_name: &str) -> Result<(), String> { Ok(()) }
    async fn check_thumbnail_exists(&self, _key: &str) -> Result<bool, String> { Ok(false) }
    async fn upload_thumbnail_from_path(&self, _thumbnail_path: &std::path::Path, _key: &str) -> Result<(), String> { Ok(()) }
    async fn generate_thumbnail_url(&self, _key: &str) -> Result<String, String> { Ok("".to_string()) }
}

pub fn create_test_state() -> Arc<PresentationState> {
    let auth_verifier = Arc::new(MockAuth);
    let video_repo = Arc::new(MockVideo);
    let download_repo = Arc::new(InMemoryDownloadRepository::new());
    let client = reqwest::Client::new();

    let verify_token_use_case = Arc::new(VerifyTokenUseCase::new(auth_verifier, "admin@example.com".to_string()));
    let list_videos_use_case = Arc::new(ListVideosUseCase::new(video_repo.clone()));
    let get_upload_url_use_case = Arc::new(GetUploadUrlUseCase::new(video_repo.clone()));
    let get_stream_url_use_case = Arc::new(GetStreamUrlUseCase::new(video_repo.clone()));
    let delete_video_use_case = Arc::new(DeleteVideoUseCase::new(video_repo.clone()));
    let rename_video_use_case = Arc::new(RenameVideoUseCase::new(video_repo.clone()));
    let list_downloads_use_case = Arc::new(ListDownloadsUseCase::new(download_repo.clone()));
    let dismiss_download_use_case = Arc::new(DismissDownloadUseCase::new(download_repo.clone()));
    let upload_from_url_use_case = Arc::new(UploadFromUrlUseCase::new(download_repo, video_repo.clone(), client));
    let get_thumbnail_use_case = Arc::new(GetThumbnailUseCase::new(video_repo));

    Arc::new(PresentationState {
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
        google_client_id: "test-client-id".to_string(),
        csp_header_value: "default-src 'self'; media-src 'self' blob: https://test-bucket.r2.cloudflarestorage.com;".to_string(),
    })
}
