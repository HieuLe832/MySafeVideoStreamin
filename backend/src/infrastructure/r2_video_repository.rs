use crate::domain::models::VideoInfo as DomainVideoInfo;
use crate::domain::ports::video_repository::VideoRepository;
use crate::r2::R2Service;
use std::path::Path;
#[axum::async_trait]
impl VideoRepository for R2Service {
    async fn get_total_storage_size(&self) -> Result<u64, String> {
        self.get_total_storage_size_cached().await
    }

    async fn list_videos(&self, force_refresh: bool) -> Result<Vec<DomainVideoInfo>, String> {
        let videos = self.list_videos_cached(force_refresh).await?;
        Ok(videos.into_iter().map(|v| DomainVideoInfo {
            key: v.key,
            original_name: v.original_name,
            size: v.size,
            uploaded_at: v.uploaded_at,
            stream_url: v.stream_url,
        }).collect())
    }

    async fn generate_upload_url(&self, original_name: &str, file_size: u64) -> Result<(String, String), String> {
        self.generate_upload_url(original_name, file_size).await
    }

    async fn generate_stream_url(&self, key: &str) -> Result<String, String> {
        self.generate_stream_url(key).await
    }

    async fn upload_file_from_path(&self, file_path: &Path, original_name: &str) -> Result<String, String> {
        self.upload_file_from_path(file_path, original_name).await
    }

    async fn delete_video(&self, key: &str) -> Result<(), String> {
        self.delete_video(key).await
    }

    async fn rename_video(&self, key: &str, new_name: &str) -> Result<(), String> {
        self.rename_video(key, new_name).await
    }

    async fn check_thumbnail_exists(&self, key: &str) -> Result<bool, String> {
        self.check_thumbnail_exists(key).await
    }

    async fn upload_thumbnail_from_path(&self, thumbnail_path: &Path, key: &str) -> Result<(), String> {
        self.upload_thumbnail_from_path(thumbnail_path, key).await
    }

    async fn generate_thumbnail_url(&self, key: &str) -> Result<String, String> {
        self.generate_thumbnail_url(key).await
    }
}
