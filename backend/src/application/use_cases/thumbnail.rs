use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::{RwLock, Semaphore};
use tracing::{info, error, warn};
use crate::domain::ports::video_repository::VideoRepository;

pub struct GetThumbnailUseCase {
    video_repository: Arc<dyn VideoRepository>,
    ffmpeg_semaphore: Semaphore,
    existing_thumbnails: RwLock<HashSet<String>>,
}

impl GetThumbnailUseCase {
    pub fn new(video_repository: Arc<dyn VideoRepository>) -> Self {
        Self { 
            video_repository,
            ffmpeg_semaphore: Semaphore::new(3), // Limit to 3 concurrent ffmpeg operations
            existing_thumbnails: RwLock::new(HashSet::new()),
        }
    }

    pub async fn execute(&self, key: &str) -> Result<String, String> {
        // 1. Check in-memory cache first (extremely fast, avoids R2 network checks)
        {
            let cache = self.existing_thumbnails.read().await;
            if cache.contains(key) {
                return self.video_repository.generate_thumbnail_url(key).await;
            }
        }

        // 2. Check if thumbnail exists in R2
        if self.video_repository.check_thumbnail_exists(key).await.unwrap_or(false) {
            info!("Thumbnail already exists in R2 for key: {}", key);
            // Cache it in-memory so subsequent requests don't hit R2
            let mut cache = self.existing_thumbnails.write().await;
            cache.insert(key.to_string());
            return self.video_repository.generate_thumbnail_url(key).await;
        }

        // 3. Acquire a permit to limit concurrent ffmpeg extraction
        let _permit = self.ffmpeg_semaphore.acquire().await
            .map_err(|e| format!("Failed to acquire ffmpeg permit: {}", e))?;

        // Double check cache and R2 in case another thread generated it while we were waiting
        {
            let cache = self.existing_thumbnails.read().await;
            if cache.contains(key) {
                return self.video_repository.generate_thumbnail_url(key).await;
            }
        }
        if self.video_repository.check_thumbnail_exists(key).await.unwrap_or(false) {
            let mut cache = self.existing_thumbnails.write().await;
            cache.insert(key.to_string());
            return self.video_repository.generate_thumbnail_url(key).await;
        }

        // 4. Check if ffmpeg is available
        let ffmpeg_check = tokio::process::Command::new("ffmpeg")
            .arg("-version")
            .output()
            .await;

        if let Err(e) = ffmpeg_check {
            warn!("ffmpeg is not available. Skipping thumbnail generation for {}. Error: {:?}", key, e);
            return Err("ffmpeg is not installed on this system".to_string());
        }

        info!("Thumbnail does not exist. Generating thumbnail on-the-fly for key: {}", key);

        // 5. Get presigned stream URL for original video
        let stream_url = self.video_repository.generate_stream_url(key).await?;

        // 6. Use system temporary folder (guarantees write permissions in environments like Cloud Run)
        let temp_dir = std::env::temp_dir();

        // Generate a unique, safe temporary filename based on timestamp to avoid any special characters
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_thumb_path = temp_dir.join(format!("thumb_lazy_{}.jpg", timestamp));

        // 7. Run ffmpeg to extract frame at second 2
        let mut ffmpeg_res = tokio::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-ss", "2",
                "-i", &stream_url,
                "-vframes", "1",
                "-q:v", "4",
                temp_thumb_path.to_str().unwrap_or(""),
            ])
            .output()
            .await;

        // Fallback to second 0 if second 2 fails (e.g. video is very short)
        if ffmpeg_res.is_err() || !ffmpeg_res.as_ref().unwrap().status.success() {
            info!("ffmpeg extraction at ss 2 failed or errored, trying ss 0 for key: {}", key);
            ffmpeg_res = tokio::process::Command::new("ffmpeg")
                .args([
                    "-y",
                    "-ss", "0",
                    "-i", &stream_url,
                    "-vframes", "1",
                    "-q:v", "4",
                    temp_thumb_path.to_str().unwrap_or(""),
                ])
                .output()
                .await;
        }

        match ffmpeg_res {
            Ok(output) if output.status.success() => {
                info!("Successfully extracted frame via ffmpeg for key: {}", key);

                // 8. Upload generated thumbnail to R2
                let upload_res = self.video_repository.upload_thumbnail_from_path(&temp_thumb_path, key).await;

                // Remove temp file
                let _ = tokio::fs::remove_file(&temp_thumb_path).await;

                match upload_res {
                    Ok(_) => {
                        // Add to in-memory cache
                        let mut cache = self.existing_thumbnails.write().await;
                        cache.insert(key.to_string());
                        
                        // Generate presigned URL for new thumbnail
                        self.video_repository.generate_thumbnail_url(key).await
                    }
                    Err(e) => {
                        error!("Failed to upload generated thumbnail for key {}: {}", key, e);
                        Err(e)
                    }
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!("ffmpeg failed to extract frame. Stderr: {}", stderr);
                let _ = tokio::fs::remove_file(&temp_thumb_path).await;
                Err(format!("ffmpeg process returned error status. Stderr: {}", stderr))
            }
            Err(e) => {
                error!("Failed to start ffmpeg process: {:?}", e);
                let _ = tokio::fs::remove_file(&temp_thumb_path).await;
                Err(format!("Failed to run ffmpeg: {}", e))
            }
        }
    }
}
