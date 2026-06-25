use crate::domain::models::VideoInfo;
use std::path::Path;

#[axum::async_trait]
pub trait VideoRepository: Send + Sync {
    async fn get_total_storage_size(&self) -> Result<u64, String>;
    async fn list_videos(&self, force_refresh: bool) -> Result<Vec<VideoInfo>, String>;
    async fn generate_upload_url(&self, original_name: &str, file_size: u64) -> Result<(String, String), String>;
    async fn generate_stream_url(&self, key: &str) -> Result<String, String>;
    async fn upload_file_from_path(&self, file_path: &Path, original_name: &str) -> Result<String, String>;
    async fn delete_video(&self, key: &str) -> Result<(), String>;
    async fn rename_video(&self, key: &str, new_name: &str) -> Result<(), String>;
    async fn check_thumbnail_exists(&self, key: &str) -> Result<bool, String>;
    async fn upload_thumbnail_from_path(&self, thumbnail_path: &Path, key: &str) -> Result<(), String>;
    async fn generate_thumbnail_url(&self, key: &str) -> Result<String, String>;
}
