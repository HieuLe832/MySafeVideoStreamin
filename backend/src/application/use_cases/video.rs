use std::sync::Arc;
use crate::domain::models::VideoInfo;
use crate::domain::ports::video_repository::VideoRepository;

pub struct ListVideosUseCase {
    video_repository: Arc<dyn VideoRepository>,
}

impl ListVideosUseCase {
    pub fn new(video_repository: Arc<dyn VideoRepository>) -> Self {
        Self { video_repository }
    }

    pub async fn execute(&self, force_refresh: bool) -> Result<(Vec<VideoInfo>, u64, u64), String> {
        let videos = self.video_repository.list_videos(force_refresh).await?;
        let total_used = self.video_repository.get_total_storage_size().await.unwrap_or(0);
        let max_limit_bytes = 9 * 1024 * 1024 * 1024; // 9 GB
        Ok((videos, total_used, max_limit_bytes))
    }
}

pub struct GetUploadUrlUseCase {
    video_repository: Arc<dyn VideoRepository>,
}

impl GetUploadUrlUseCase {
    pub fn new(video_repository: Arc<dyn VideoRepository>) -> Self {
        Self { video_repository }
    }

    pub async fn execute(&self, original_name: &str, file_size: u64) -> Result<(String, String), String> {
        self.video_repository.generate_upload_url(original_name, file_size).await
    }
}

pub struct GetStreamUrlUseCase {
    video_repository: Arc<dyn VideoRepository>,
}

impl GetStreamUrlUseCase {
    pub fn new(video_repository: Arc<dyn VideoRepository>) -> Self {
        Self { video_repository }
    }

    pub async fn execute(&self, key: &str) -> Result<String, String> {
        self.video_repository.generate_stream_url(key).await
    }
}

pub struct DeleteVideoUseCase {
    video_repository: Arc<dyn VideoRepository>,
}

impl DeleteVideoUseCase {
    pub fn new(video_repository: Arc<dyn VideoRepository>) -> Self {
        Self { video_repository }
    }

    pub async fn execute(&self, key: &str) -> Result<(), String> {
        self.video_repository.delete_video(key).await
    }
}

pub struct RenameVideoUseCase {
    video_repository: Arc<dyn VideoRepository>,
}

impl RenameVideoUseCase {
    pub fn new(video_repository: Arc<dyn VideoRepository>) -> Self {
        Self { video_repository }
    }

    pub async fn execute(&self, key: &str, new_name: &str) -> Result<(), String> {
        self.video_repository.rename_video(key, new_name).await
    }
}
