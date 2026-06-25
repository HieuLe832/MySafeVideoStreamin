use crate::application::use_cases::download::UploadFromUrlUseCase;
use crate::infrastructure::memory_download_repository::InMemoryDownloadRepository;
use crate::domain::ports::video_repository::VideoRepository;
use crate::domain::ports::download_repository::DownloadRepository;
use crate::domain::models::VideoInfo;
use std::sync::Arc;
use std::path::Path;
struct MockVideoRepository;

#[axum::async_trait]
impl VideoRepository for MockVideoRepository {
    async fn get_total_storage_size(&self) -> Result<u64, String> {
        Ok(0)
    }
    async fn list_videos(&self, _force_refresh: bool) -> Result<Vec<VideoInfo>, String> {
        Ok(vec![])
    }
    async fn generate_upload_url(&self, _original_name: &str, _file_size: u64) -> Result<(String, String), String> {
        Ok(("url".to_string(), "key".to_string()))
    }
    async fn generate_stream_url(&self, _key: &str) -> Result<String, String> {
        Ok("stream".to_string())
    }
    async fn upload_file_from_path(&self, _file_path: &Path, _original_name: &str) -> Result<String, String> {
        Ok("key".to_string())
    }
    async fn delete_video(&self, _key: &str) -> Result<(), String> {
        Ok(())
    }
    async fn rename_video(&self, _key: &str, _new_name: &str) -> Result<(), String> {
        Ok(())
    }
    async fn check_thumbnail_exists(&self, _key: &str) -> Result<bool, String> {
        Ok(false)
    }
    async fn upload_thumbnail_from_path(&self, _thumbnail_path: &Path, _key: &str) -> Result<(), String> {
        Ok(())
    }
    async fn generate_thumbnail_url(&self, _key: &str) -> Result<String, String> {
        Ok("".to_string())
    }
}

#[tokio::test]
async fn test_upload_from_url_empty() {
    let download_repo = Arc::new(InMemoryDownloadRepository::new());
    let video_repo = Arc::new(MockVideoRepository);
    let client = reqwest::Client::new();
    let use_case = Arc::new(UploadFromUrlUseCase::new(download_repo, video_repo, client));

    let result = use_case.execute("   ").await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "URL không được trống.");
}

#[tokio::test]
async fn test_upload_from_url_invalid_format() {
    let download_repo = Arc::new(InMemoryDownloadRepository::new());
    let video_repo = Arc::new(MockVideoRepository);
    let client = reqwest::Client::new();
    let use_case = Arc::new(UploadFromUrlUseCase::new(download_repo, video_repo, client));

    let result = use_case.execute("not-a-url").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("URL không hợp lệ"));
}

#[tokio::test]
async fn test_upload_from_url_invalid_scheme() {
    let download_repo = Arc::new(InMemoryDownloadRepository::new());
    let video_repo = Arc::new(MockVideoRepository);
    let client = reqwest::Client::new();
    let use_case = Arc::new(UploadFromUrlUseCase::new(download_repo, video_repo, client));

    let result = use_case.execute("ftp://example.com/movie.mp4").await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Chỉ hỗ trợ giao thức HTTP hoặc HTTPS.");
}

#[tokio::test]
async fn test_upload_from_url_ssrf_ipv4_loopback() {
    let download_repo = Arc::new(InMemoryDownloadRepository::new());
    let video_repo = Arc::new(MockVideoRepository);
    let client = reqwest::Client::new();
    let use_case = Arc::new(UploadFromUrlUseCase::new(download_repo, video_repo, client));

    let result = use_case.execute("http://127.0.0.1/movie.mp4").await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Địa chỉ IP bị cấm vì lý do bảo mật (SSRF).");
}

#[tokio::test]
async fn test_upload_from_url_ssrf_ipv4_private() {
    let download_repo = Arc::new(InMemoryDownloadRepository::new());
    let video_repo = Arc::new(MockVideoRepository);
    let client = reqwest::Client::new();
    let use_case = Arc::new(UploadFromUrlUseCase::new(download_repo, video_repo, client));

    let result = use_case.execute("http://192.168.1.50/movie.mp4").await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Địa chỉ IP bị cấm vì lý do bảo mật (SSRF).");
}

#[tokio::test]
async fn test_upload_from_url_success_init() {
    let download_repo = Arc::new(InMemoryDownloadRepository::new());
    let video_repo = Arc::new(MockVideoRepository);
    let client = reqwest::Client::new();
    let use_case = Arc::new(UploadFromUrlUseCase::new(download_repo.clone(), video_repo, client));

    // Using a domain that resolves to public IPs (e.g. 1.1.1.1 or 8.8.8.8)
    let result = use_case.execute("https://1.1.1.1/movie.mp4").await;
    assert!(result.is_ok());
    
    let task_id = result.unwrap();
    assert!(!task_id.is_empty());

    // Verify task is added to repo
    let downloads = download_repo.list_downloads();
    assert_eq!(downloads.len(), 1);
    assert_eq!(downloads[0].id, task_id);
    assert_eq!(downloads[0].status, "Downloading");
}

#[tokio::test]
async fn test_download_task_abort() {
    let download_repo = InMemoryDownloadRepository::new();
    let task_id = "test_cancel_id".to_string();

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    let handle = tokio::spawn(async move {
        // Wait until aborted
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        let _ = tx.send(());
    });

    download_repo.register_handle(&task_id, handle);

    // Call remove_download, which should abort the registered handle
    download_repo.remove_download(&task_id);

    // Verify the task was aborted immediately (sender dropped, rx resolves to Err)
    let rx_result = tokio::time::timeout(std::time::Duration::from_millis(500), rx).await;
    assert!(rx_result.is_ok()); // Resolved within 500ms
    assert!(rx_result.unwrap().is_err()); // Resolved to Err because Sender was dropped on abort
}
