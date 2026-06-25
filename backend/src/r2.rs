use aws_config::SdkConfig;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region, SharedCredentialsProvider};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client as S3Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn, error};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Instant;

const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024 * 1024; // 2 GB
const MAX_TOTAL_STORAGE: u64 = 9 * 1024 * 1024 * 1024; // 9 GB

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VideoInfo {
    pub key: String,
    pub original_name: String,
    pub size: u64,
    pub uploaded_at: String,
    pub stream_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BucketMetadata {
    #[serde(default)]
    pub video_names: HashMap<String, String>,
}

struct VideoListCache {
    videos: Vec<VideoInfo>,
    total_size: u64,
    fetched_at: Instant,
}

#[derive(Clone)]
pub struct R2Service {
    client: S3Client,
    bucket_name: String,
    list_cache: Arc<RwLock<Option<VideoListCache>>>,
}

impl R2Service {
    pub fn new(account_id: &str, access_key_id: &str, secret_access_key: &str, bucket_name: &str) -> Self {
        let credentials = Credentials::new(
            access_key_id,
            secret_access_key,
            None,
            None,
            "r2-provider",
        );

        let endpoint_url = format!("https://{}.r2.cloudflarestorage.com", account_id);
        
        let sdk_config = SdkConfig::builder()
            .credentials_provider(SharedCredentialsProvider::new(credentials))
            .endpoint_url(endpoint_url)
            .region(Region::new("auto"))
            .behavior_version(BehaviorVersion::latest())
            .build();

        let client = S3Client::new(&sdk_config);

        Self {
            client,
            bucket_name: bucket_name.to_string(),
            list_cache: Arc::new(RwLock::new(None)),
        }
    }

    async fn load_metadata(&self) -> BucketMetadata {
        let response_res = self.client
            .get_object()
            .bucket(&self.bucket_name)
            .key(".metadata.json")
            .send()
            .await;

        match response_res {
            Ok(output) => {
                let bytes_res = output.body.collect().await;
                match bytes_res {
                    Ok(bytes) => {
                        serde_json::from_slice(&bytes.into_bytes())
                            .unwrap_or_default()
                    }
                    Err(e) => {
                        warn!("Error reading metadata file bytes: {:?}", e);
                        BucketMetadata::default()
                    }
                }
            }
            Err(e) => {
                info!("No metadata file found or error fetching it: {:?}", e);
                BucketMetadata::default()
            }
        }
    }

    async fn save_metadata(&self, metadata: &BucketMetadata) -> Result<(), String> {
        let json_bytes = serde_json::to_vec(metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

        let body = aws_sdk_s3::primitives::ByteStream::from(json_bytes);

        self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(".metadata.json")
            .body(body)
            .send()
            .await
            .map_err(|e| {
                error!("Error saving metadata.json: {:?}", e);
                format!("Failed to save metadata to R2: {}", e)
            })?;

        Ok(())
    }

    async fn invalidate_cache(&self) {
        let mut cache = self.list_cache.write().await;
        *cache = None;
        info!("Invalidated video list cache.");
    }

    async fn get_cached_or_fetch(&self, force_refresh: bool) -> Result<(Vec<VideoInfo>, u64), String> {
        let now = Instant::now();
        
        if !force_refresh {
            let cache = self.list_cache.read().await;
            if let Some(ref c) = *cache {
                if now.duration_since(c.fetched_at) < Duration::from_secs(15) {
                    return Ok((c.videos.clone(), c.total_size));
                }
            }
        }

        info!("Cache invalid, expired, or force refreshed. Fetching fresh data from R2...");
        
        let mut videos = Vec::new();
        let mut total_size: u64 = 0;
        
        let mut response = self.client
            .list_objects_v2()
            .bucket(&self.bucket_name)
            .send()
            .await
            .map_err(|e| {
                error!("Error listing objects: {:?}", e);
                format!("Failed to list objects from R2: {}", e)
            })?;

        let metadata = self.load_metadata().await;

        loop {
            for object in response.contents() {
                let key = object.key().unwrap_or("").to_string();
                if key.is_empty() {
                    continue;
                }
                
                let size = object.size().unwrap_or(0) as u64;

                if !key.starts_with('.') {
                    total_size += size;
                }

                if key.starts_with('.') || key.starts_with("thumbnails/") {
                    continue;
                }

                let uploaded_at = match object.last_modified() {
                    Some(dt) => dt.to_string(),
                    None => "Unknown".to_string(),
                };

                let original_name = match metadata.video_names.get(&key) {
                    Some(name) => name.clone(),
                    None => match key.find('_') {
                        Some(idx) => {
                            let encoded_part = &key[idx + 1..];
                            urlencoding::decode(encoded_part)
                                .map(|cow| cow.into_owned())
                                .unwrap_or_else(|_| key.clone())
                        }
                        None => key.clone(),
                    }
                };

                let stream_url = match self.generate_stream_url(&key).await {
                    Ok(url) => url,
                    Err(_) => "".to_string(),
                };

                videos.push(VideoInfo {
                    key,
                    original_name,
                    size,
                    uploaded_at,
                    stream_url,
                });
            }

            if response.is_truncated().unwrap_or(false) {
                let next_token = response.next_continuation_token().map(|s| s.to_string());
                if next_token.is_none() {
                    break;
                }
                
                response = self.client
                    .list_objects_v2()
                    .bucket(&self.bucket_name)
                    .continuation_token(next_token.unwrap())
                    .send()
                    .await
                    .map_err(|e| {
                        error!("Error continuing list objects: {:?}", e);
                        format!("Failed to continue listing R2 objects: {}", e)
                    })?;
            } else {
                break;
            }
        }

        videos.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at));

        let mut cache = self.list_cache.write().await;
        *cache = Some(VideoListCache {
            videos: videos.clone(),
            total_size,
            fetched_at: now,
        });

        Ok((videos, total_size))
    }

    /// Lấy tổng dung lượng hiện tại của tất cả các file trong R2 bucket (Sử dụng cache)
    pub async fn get_total_storage_size_cached(&self) -> Result<u64, String> {
        let (_, total_size) = self.get_cached_or_fetch(false).await?;
        Ok(total_size)
    }

    /// Liệt kê danh sách video trong R2 bucket kèm metadata (Sử dụng cache)
    pub async fn list_videos_cached(&self, force_refresh: bool) -> Result<Vec<VideoInfo>, String> {
        let (videos, _) = self.get_cached_or_fetch(force_refresh).await?;
        Ok(videos)
    }


    /// Tạo Presigned URL để upload file trực tiếp từ Client lên R2
    pub async fn generate_upload_url(&self, original_name: &str, file_size: u64) -> Result<(String, String), String> {
        // 1. Kiểm tra phần mở rộng file (Extension validation)
        let allowed_extensions = ["mp4", "mkv", "webm", "avi", "mov", "flv", "wmv", "m4v", "mpeg", "mpg"];
        let extension = std::path::Path::new(original_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        if !allowed_extensions.contains(&extension.as_str()) {
            warn!("Rejected upload request for file with invalid extension: {}", original_name);
            return Err("Định dạng file không được hỗ trợ. Chỉ cho phép các file video (mp4, mkv, webm, avi, mov, v.v.).".to_string());
        }

        // 2. Kiểm tra giới hạn file đơn lẻ
        if file_size > MAX_FILE_SIZE {
            warn!("File size {} exceeds limit of 2GB", file_size);
            return Err("Kích thước file vượt quá giới hạn 2 GB.".to_string());
        }

        // 3. Kiểm tra tổng dung lượng hiện tại (Sử dụng cache để tối ưu)
        let total_size = self.get_total_storage_size_cached().await?;
        if total_size + file_size > MAX_TOTAL_STORAGE {
            warn!("Storage limit exceeded. Current: {}, New: {}, Max: {}", total_size, file_size, MAX_TOTAL_STORAGE);
            return Err("Không đủ dung lượng trống. Tổng lưu trữ sẽ vượt quá giới hạn 9 GB.".to_string());
        }

        // 4. Tạo key duy nhất có cấu trúc: [timestamp]_[url_encoded_original_name]
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let encoded_name = urlencoding::encode(original_name);
        let key = format!("{}_{}", timestamp, encoded_name);

        // 5. Tạo Presigned PUT URL
        let presigning_config = PresigningConfig::builder()
            .expires_in(Duration::from_secs(3600)) // Có hiệu lực trong 1 giờ
            .build()
            .map_err(|e| format!("Failed to create presigning config: {}", e))?;

        let presigned_req = self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(&key)
            .content_length(file_size as i64)
            .presigned(presigning_config)
            .await
            .map_err(|e| {
                error!("Error generating presigned PUT URL: {:?}", e);
                format!("Failed to generate upload URL: {}", e)
            })?;

        let upload_url = presigned_req.uri().to_string();
        info!("Generated presigned upload URL for key: {}", key);

        // Invalidate cache since we will get a new file soon
        self.invalidate_cache().await;

        Ok((upload_url, key))
    }

    /// Tạo Presigned URL để client stream video trực tiếp từ R2
    pub async fn generate_stream_url(&self, key: &str) -> Result<String, String> {
        let presigning_config = PresigningConfig::builder()
            .expires_in(Duration::from_secs(7200)) // Link xem có hiệu lực trong 2 giờ
            .build()
            .map_err(|e| format!("Failed to create presigning config: {}", e))?;

        let presigned_req = self.client
            .get_object()
            .bucket(&self.bucket_name)
            .key(key)
            .presigned(presigning_config)
            .await
            .map_err(|e| {
                error!("Error generating presigned GET URL for key {}: {:?}", key, e);
                format!("Failed to generate stream URL: {}", e)
            })?;

        let stream_url = presigned_req.uri().to_string();
        Ok(stream_url)
    }

    /// Tải file từ đường dẫn local và upload lên R2 bucket
    pub async fn upload_file_from_path(&self, file_path: &std::path::Path, original_name: &str) -> Result<String, String> {
        // 1. Kiểm tra phần mở rộng file (Extension validation)
        let allowed_extensions = ["mp4", "mkv", "webm", "avi", "mov", "flv", "wmv", "m4v", "mpeg", "mpg"];
        let extension = std::path::Path::new(original_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        if !allowed_extensions.contains(&extension.as_str()) {
            warn!("Rejected upload request for file with invalid extension: {}", original_name);
            return Err("Định dạng file không được hỗ trợ. Chỉ cho phép các file video (mp4, mkv, webm, avi, mov, v.v.).".to_string());
        }

        // 2. Lấy kích thước file thực tế
        let metadata = std::fs::metadata(file_path)
            .map_err(|e| format!("Không thể đọc thông tin file tạm: {}", e))?;
        let file_size = metadata.len();

        // 3. Kiểm tra giới hạn file đơn lẻ
        if file_size > MAX_FILE_SIZE {
            return Err("Kích thước file vượt quá giới hạn 2 GB.".to_string());
        }

        // 4. Kiểm tra tổng dung lượng hiện tại
        let total_size = self.get_total_storage_size_cached().await?;
        if total_size + file_size > MAX_TOTAL_STORAGE {
            return Err("Không đủ dung lượng trống. Tổng lưu trữ sẽ vượt quá giới hạn 9 GB.".to_string());
        }

        // 5. Tạo key duy nhất có cấu trúc: [timestamp]_[url_encoded_original_name]
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let encoded_name = urlencoding::encode(original_name);
        let key = format!("{}_{}", timestamp, encoded_name);

        // 6. Tạo ByteStream từ file
        let body = aws_sdk_s3::primitives::ByteStream::from_path(file_path).await
            .map_err(|e| format!("Không thể đọc file thành ByteStream: {}", e))?;

        // 7. Gọi put_object tải lên R2
        self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(&key)
            .body(body)
            .content_length(file_size as i64)
            .send()
            .await
            .map_err(|e| {
                error!("Lỗi khi tải file từ ổ đĩa lên R2: {:?}", e);
                format!("Tải file lên R2 thất bại: {}", e)
            })?;

        info!("Successfully uploaded video to R2 with key: {}", key);
        
        // Invalidate cache
        self.invalidate_cache().await;

        Ok(key)
    }

    /// Xóa video khỏi R2 bucket
    pub async fn delete_video(&self, key: &str) -> Result<(), String> {
        self.client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                error!("Error deleting object {}: {:?}", key, e);
                format!("Failed to delete video: {}", e)
            })?;

        // Xóa cả thumbnail tương ứng nếu có
        let thumb_key = format!("thumbnails/{}.jpg", key);
        let _ = self.client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(&thumb_key)
            .send()
            .await;

        // Cập nhật metadata xóa entry
        let mut metadata = self.load_metadata().await;
        if metadata.video_names.remove(key).is_some() {
            let _ = self.save_metadata(&metadata).await;
        }

        info!("Deleted object with key: {} successfully", key);
        
        // Invalidate cache
        self.invalidate_cache().await;

        Ok(())
    }

    /// Đổi tên video trong R2 bucket bằng cách cập nhật tên hiển thị trong file metadata
    pub async fn rename_video(&self, key: &str, new_name: &str) -> Result<(), String> {
        if new_name.trim().is_empty() {
            return Err("Tên file mới không được để trống.".to_string());
        }

        let mut final_new_name = new_name.trim().to_string();

        // Kiểm tra phần mở rộng file (Extension validation)
        let allowed_extensions = ["mp4", "mkv", "webm", "avi", "mov", "flv", "wmv", "m4v", "mpeg", "mpg"];
        let extension = std::path::Path::new(&final_new_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        if !allowed_extensions.contains(&extension.as_str()) {
            // Tự động lấy phần mở rộng từ key cũ (ví dụ: key chứa đuôi .mp4 ở cuối)
            let old_ext = std::path::Path::new(key)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("mp4")
                .to_lowercase();
            
            final_new_name = format!("{}.{}", final_new_name, old_ext);
        }

        let mut metadata = self.load_metadata().await;
        metadata.video_names.insert(key.to_string(), final_new_name.clone());
        self.save_metadata(&metadata).await?;

        info!("Successfully renamed key {} to '{}' in metadata", key, final_new_name);
        
        // Invalidate cache
        self.invalidate_cache().await;

        Ok(())
    }

    /// Kiểm tra xem ảnh thumbnail đã tồn tại trên R2 chưa
    pub async fn check_thumbnail_exists(&self, key: &str) -> Result<bool, String> {
        let thumb_key = format!("thumbnails/{}.jpg", key);
        let res = self.client
            .head_object()
            .bucket(&self.bucket_name)
            .key(&thumb_key)
            .send()
            .await;

        match res {
            Ok(_) => Ok(true),
            Err(e) => {
                info!("Thumbnail {} not found or error checking: {:?}", thumb_key, e);
                Ok(false)
            }
        }
    }

    /// Tải ảnh thumbnail lên R2
    pub async fn upload_thumbnail_from_path(&self, thumbnail_path: &std::path::Path, key: &str) -> Result<(), String> {
        let thumb_key = format!("thumbnails/{}.jpg", key);
        
        let body = aws_sdk_s3::primitives::ByteStream::from_path(thumbnail_path).await
            .map_err(|e| format!("Không thể đọc file thumbnail tạm: {}", e))?;

        self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(&thumb_key)
            .body(body)
            .content_type("image/jpeg")
            .send()
            .await
            .map_err(|e| {
                error!("Lỗi khi tải thumbnail lên R2: {:?}", e);
                format!("Tải thumbnail lên R2 thất bại: {}", e)
            })?;

        info!("Successfully uploaded thumbnail to R2 with key: {}", thumb_key);
        
        // Invalidate cache
        self.invalidate_cache().await;

        Ok(())
    }

    /// Tạo presigned GET URL cho ảnh thumbnail
    pub async fn generate_thumbnail_url(&self, key: &str) -> Result<String, String> {
        let thumb_key = format!("thumbnails/{}.jpg", key);
        let presigning_config = PresigningConfig::builder()
            .expires_in(Duration::from_secs(7200)) // Link xem có hiệu lực trong 2 giờ
            .build()
            .map_err(|e| format!("Failed to create presigning config: {}", e))?;

        let presigned_req = self.client
            .get_object()
            .bucket(&self.bucket_name)
            .key(&thumb_key)
            .presigned(presigning_config)
            .await
            .map_err(|e| {
                error!("Error generating presigned GET URL for thumbnail {}: {:?}", thumb_key, e);
                format!("Failed to generate thumbnail stream URL: {}", e)
            })?;

        let stream_url = presigned_req.uri().to_string();
        Ok(stream_url)
    }
}
