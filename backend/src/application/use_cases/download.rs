use std::sync::Arc;
use tracing::{info, warn, error};
use crate::domain::models::ActiveDownload;
use crate::domain::ports::download_repository::DownloadRepository;
use crate::domain::ports::video_repository::VideoRepository;
use crate::utils::{is_private_ip, get_safe_filename, TempFileGuard};

pub struct ListDownloadsUseCase {
    download_repository: Arc<dyn DownloadRepository>,
}

impl ListDownloadsUseCase {
    pub fn new(download_repository: Arc<dyn DownloadRepository>) -> Self {
        Self { download_repository }
    }

    pub fn execute(&self) -> Vec<ActiveDownload> {
        self.download_repository.list_downloads()
    }
}

pub struct DismissDownloadUseCase {
    download_repository: Arc<dyn DownloadRepository>,
}

impl DismissDownloadUseCase {
    pub fn new(download_repository: Arc<dyn DownloadRepository>) -> Self {
        Self { download_repository }
    }

    pub fn execute(&self, id: &str) -> bool {
        self.download_repository.remove_download(id).is_some()
    }
}

pub struct UploadFromUrlUseCase {
    download_repository: Arc<dyn DownloadRepository>,
    video_repository: Arc<dyn VideoRepository>,
    download_client: reqwest::Client,
}

impl UploadFromUrlUseCase {
    pub fn new(
        download_repository: Arc<dyn DownloadRepository>,
        video_repository: Arc<dyn VideoRepository>,
        download_client: reqwest::Client,
    ) -> Self {
        Self {
            download_repository,
            video_repository,
            download_client,
        }
    }

    pub async fn execute(self: &Arc<Self>, url_str: &str) -> Result<String, String> {
        let url_str = url_str.trim();
        if url_str.is_empty() {
            return Err("URL không được trống.".into());
        }

        // Parse URL cơ bản
        let parsed_url = reqwest::Url::parse(url_str)
            .map_err(|e| format!("URL không hợp lệ: {}", e))?;

        if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
            return Err("Chỉ hỗ trợ giao thức HTTP hoặc HTTPS.".into());
        }

        // Validate DNS cho host ban đầu để bắt SSRF ngay lập tức
        let host = parsed_url.host_str()
            .ok_or_else(|| "URL không có host.".to_string())?;
        let port = parsed_url.port().unwrap_or(if parsed_url.scheme() == "https" { 443 } else { 80 });

        let addrs = tokio::net::lookup_host(format!("{}:{}", host, port)).await
            .map_err(|e| format!("Không thể phân giải tên miền: {}", e))?;

        let mut has_resolved_ip = false;
        for addr in addrs {
            has_resolved_ip = true;
            if is_private_ip(addr.ip()) {
                return Err("Địa chỉ IP bị cấm vì lý do bảo mật (SSRF).".into());
            }
        }

        if !has_resolved_ip {
            return Err("Tên miền không trỏ tới địa chỉ IP nào.".into());
        }

        // Tạo ID tác vụ tải ngầm duy nhất dựa trên thời gian
        let task_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string();

        let safe_filename = get_safe_filename(&parsed_url, None);

        // Ghi nhận tác vụ vào Repository dưới dạng Pending
        let initial_download = ActiveDownload {
            id: task_id.clone(),
            url: url_str.to_string(),
            filename: safe_filename.clone(),
            status: "Pending".to_string(),
            error: None,
            downloaded_bytes: 0,
            total_bytes: None,
        };

        self.download_repository.add_download(initial_download);

        // Kích hoạt xử lý hàng đợi
        self.process_queue();

        // Trả về Task ID
        Ok(task_id)
    }

    pub fn process_queue(self: &Arc<Self>) {
        let downloads = self.download_repository.list_downloads();

        // Đếm số lượng tác vụ đang hoạt động
        let active_count = downloads.iter()
            .filter(|d| d.status == "Downloading" || d.status == "Uploading")
            .count();

        if active_count >= 1 {
            info!("Queue: Có {} tác vụ đang chạy. Tiếp tục chờ.", active_count);
            return;
        }

        // Tìm tác vụ ở trạng thái Pending tiếp theo, ưu tiên theo ID (timestamp)
        let mut pending_tasks: Vec<_> = downloads.into_iter()
            .filter(|d| d.status == "Pending")
            .collect();

        pending_tasks.sort_by(|a, b| a.id.cmp(&b.id));

        if let Some(next_task) = pending_tasks.first() {
            let task_id = next_task.id.clone();
            let url_str = next_task.url.clone();
            info!("Queue: Bắt đầu tải tác vụ chờ: {}", task_id);

            // Cập nhật trạng thái sang Downloading
            self.download_repository.update_download(&task_id, Box::new(|task| {
                task.status = "Downloading".to_string();
            }));

            // Chuẩn bị các biến cho worker
            let download_repository_clone = self.download_repository.clone();
            let video_repository_clone = self.video_repository.clone();
            let download_client_clone = self.download_client.clone();
            let task_id_clone = task_id.clone();
            let parsed_url = match reqwest::Url::parse(&url_str) {
                Ok(url) => url,
                Err(_) => {
                    self.download_repository.update_download(&task_id, Box::new(|task| {
                        task.status = "Failed".to_string();
                        task.error = Some("URL không hợp lệ trong hàng đợi.".to_string());
                    }));
                    return;
                }
            };

            let use_case_clone = self.clone();
            let download_repository_clone2 = self.download_repository.clone();
            let task_id_clone2 = task_id.clone();

            let handle = tokio::spawn(async move {
                info!("Background task spawned for downloading URL: {}", parsed_url);
                match perform_background_download_worker(
                    download_repository_clone.clone(),
                    video_repository_clone,
                    download_client_clone,
                    task_id_clone.clone(),
                    parsed_url,
                ).await {
                    Ok(_) => {
                        info!("Background task {} completed successfully", task_id_clone);
                        download_repository_clone.remove_download(&task_id_clone);
                    }
                    Err(err_msg) => {
                        error!("Background task {} failed: {}", task_id_clone, err_msg);
                        download_repository_clone.update_download(&task_id_clone, Box::new(|task| {
                            task.status = "Failed".to_string();
                            task.error = Some(err_msg);
                        }));
                    }
                }
                download_repository_clone2.deregister_handle(&task_id_clone2);

                // Kích hoạt tác vụ tiếp theo
                use_case_clone.process_queue();
            });

            self.download_repository.register_handle(&task_id, handle);
        }
    }
}

// Tiến trình tải và lưu đệm video ngầm
async fn perform_background_download_worker(
    download_repository: Arc<dyn DownloadRepository>,
    video_repository: Arc<dyn VideoRepository>,
    _download_client: reqwest::Client,
    task_id: String,
    parsed_url: reqwest::Url,
) -> Result<(), String> {
    let mut current_url = parsed_url;
    let mut redirect_count = 0;
    let max_redirects = 5;

    let response = loop {
        if redirect_count >= max_redirects {
            return Err("Vượt quá số lượng redirect cho phép.".to_string());
        }

        let host = current_url.host_str()
            .ok_or_else(|| "URL không có host.".to_string())?;
        let port = current_url.port().unwrap_or(if current_url.scheme() == "https" { 443 } else { 80 });

        // Kiểm tra bảo mật DNS của host redirect
        let addrs = tokio::net::lookup_host(format!("{}:{}", host, port)).await
            .map_err(|e| format!("Không thể phân giải tên miền: {}", e))?;

        let mut has_resolved_ip = false;
        let mut checked_ip = None;
        for addr in addrs {
            has_resolved_ip = true;
            let ip = addr.ip();
            if is_private_ip(ip) {
                warn!("SSRF blocked connection to private IP: {} for host: {}", ip, host);
                return Err("Địa chỉ IP bị cấm vì lý do bảo mật (SSRF).".to_string());
            }
            if checked_ip.is_none() {
                checked_ip = Some(ip);
            }
        }

        if !has_resolved_ip {
            return Err("Tên miền không trỏ tới địa chỉ IP nào.".into());
        }

        let checked_ip = checked_ip.ok_or_else(|| "Không có IP hợp lệ.".to_string())?;

        // Khởi tạo reqwest Client động ghim host -> checked_ip (DNS Pinning) để chống SSRF TOCTOU
        let pinned_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(std::time::Duration::from_secs(15))
            .timeout(std::time::Duration::from_secs(3600)) // 60 minutes timeout for large downloads
            .no_gzip()
            .no_brotli()
            .resolve_to_addrs(host, &[std::net::SocketAddr::new(checked_ip, port)])
            .build()
            .map_err(|e| format!("Không thể khởi tạo client tải an toàn: {}", e))?;

        // Tải bằng client đã ghim IP
        let resp = pinned_client.get(current_url.clone())
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) SafeStream/1.0")
            .send()
            .await
            .map_err(|e| format!("Lỗi kết nối khi tải video: {}", e))?;

        let status = resp.status();
        if status.is_redirection() {
            if let Some(location) = resp.headers().get(reqwest::header::LOCATION) {
                let location_str = location.to_str()
                    .map_err(|_| "Header Location chứa ký tự không hợp lệ.".to_string())?;
                let next_url = current_url.join(location_str)
                    .map_err(|_| "URL redirect không hợp lệ.".to_string())?;

                if next_url.scheme() != "http" && next_url.scheme() != "https" {
                    return Err("Giao thức redirect không hỗ trợ.".to_string());
                }

                current_url = next_url;
                redirect_count += 1;
                continue;
            } else {
                return Err("Redirect nhưng thiếu header Location.".to_string());
            }
        }

        if !status.is_success() {
            return Err(format!("Yêu cầu tải video thất bại với status code: {}", status));
        }

        break resp;
    };

    let content_length = response.content_length();
    let max_single_file: u64 = 2 * 1024 * 1024 * 1024; // 2 GB
    let max_total_storage: u64 = 9 * 1024 * 1024 * 1024; // 9 GB

    // Kiểm tra dung lượng bucket hiện tại
    let current_total_size = video_repository.get_total_storage_size().await?;

    if let Some(len) = content_length {
        if len > max_single_file {
            return Err(format!("Kích thước file quá lớn: {:.2} GB. Giới hạn là 2 GB.", len as f64 / 1024.0 / 1024.0 / 1024.0));
        }
        if current_total_size + len > max_total_storage {
            return Err("Không đủ dung lượng trống. Tổng dung lượng sẽ vượt quá 9 GB.".to_string());
        }
    }

    // Trích xuất tên file chính xác từ header hoặc URL
    let content_type = response.headers().get(reqwest::header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok());
    let safe_filename = get_safe_filename(&current_url, content_type);

    // Cập nhật tên file thực tế và tổng dung lượng vào ActiveDownload
    let filename_for_task = safe_filename.clone();
    download_repository.update_download(&task_id, Box::new(move |task| {
        task.filename = filename_for_task;
        task.total_bytes = content_length;
    }));

    // Sử dụng thư mục tạm hệ thống (đảm bảo hoạt động trên các môi trường serverless như Cloud Run)
    let temp_dir = std::env::temp_dir();

    let temp_file_path = temp_dir.join(format!("temp_video_{}.tmp", task_id));

    // Khởi tạo RAII Guard cho file tạm để tránh leak khi task bị cancel giữa chừng
    let _file_guard = TempFileGuard::new(temp_file_path.clone());

    // Ghi luồng xuống đĩa
    let mut file = tokio::fs::File::create(&temp_file_path).await
        .map_err(|e| format!("Không thể tạo file tạm: {}", e))?;

    let mut downloaded_bytes: u64 = 0;
    let mut stream = response;
    let mut header_buffer = Vec::with_capacity(16);
    let mut signature_checked = false;

    while let Some(chunk) = stream.chunk().await.map_err(|e| {
        format!("Lỗi luồng tải: {}", e)
    })? {
        downloaded_bytes += chunk.len() as u64;

        if downloaded_bytes > max_single_file {
            return Err("Kích thước file thực tế vượt quá giới hạn 2 GB.".to_string());
        }
        if current_total_size + downloaded_bytes > max_total_storage {
            return Err("Không đủ dung lượng trống (Vượt quá 9 GB).".to_string());
        }

        use tokio::io::AsyncWriteExt;
        file.write_all(&chunk).await.map_err(|e| {
            format!("Lỗi ghi file tạm: {}", e)
        })?;

        // Tích lũy header bytes để kiểm tra Magic Bytes
        if !signature_checked {
            if header_buffer.len() < 16 {
                let needed = 16 - header_buffer.len();
                let to_copy = std::cmp::min(needed, chunk.len());
                header_buffer.extend_from_slice(&chunk[..to_copy]);
            }

            if header_buffer.len() >= 12 {
                if !crate::utils::is_valid_video_signature(&header_buffer) {
                    return Err("Tệp tải về không khớp định dạng video được hỗ trợ (Magic Bytes check failed).".to_string());
                }
                signature_checked = true;
            }
        }

        // Cập nhật số byte đã tải cho ActiveDownload
        download_repository.update_download(&task_id, Box::new(move |task| {
            task.downloaded_bytes = downloaded_bytes;
        }));
    }

    // Nếu tải xong file cực nhỏ (<12 bytes) mà chưa kịp kiểm tra signature
    if !signature_checked {
        if !crate::utils::is_valid_video_signature(&header_buffer) {
            return Err("Tệp tải về không khớp định dạng video được hỗ trợ (Magic Bytes check failed).".to_string());
        }
    }

    use tokio::io::AsyncWriteExt;
    file.flush().await.map_err(|e| {
        format!("Lỗi hoàn tất ghi file: {}", e)
    })?;

    // Chuyển trạng thái tác vụ thành Đang tải lên R2
    download_repository.update_download(&task_id, Box::new(move |task| {
        task.status = "Uploading".to_string();
    }));

    // Tải từ đĩa lên R2
    let upload_result = video_repository.upload_file_from_path(&temp_file_path, &safe_filename).await;

    match upload_result {
        Ok(key) => {
            // Tải video thành công, trích xuất thumbnail ngay tại đây từ file local
            let ffmpeg_check = tokio::process::Command::new("ffmpeg")
                .arg("-version")
                .output()
                .await;

            if ffmpeg_check.is_ok() {
                let thumbnail_temp_path = temp_dir.join(format!("thumb_download_{}.jpg", task_id));
                let ffmpeg_res = tokio::process::Command::new("ffmpeg")
                    .args([
                        "-y",
                        "-ss", "5",
                        "-i", temp_file_path.to_str().unwrap_or(""),
                        "-vframes", "1",
                        "-q:v", "4",
                        thumbnail_temp_path.to_str().unwrap_or(""),
                    ])
                    .output()
                    .await;

                match ffmpeg_res {
                    Ok(output) if output.status.success() => {
                        info!("Successfully generated thumbnail locally for downloaded video key: {}", key);
                        if let Err(e) = video_repository.upload_thumbnail_from_path(&thumbnail_temp_path, &key).await {
                            warn!("Failed to upload pre-generated thumbnail for key {}: {}", key, e);
                        }
                    }
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        warn!("ffmpeg failed to pre-generate thumbnail. Stderr: {}", stderr);
                    }
                    Err(e) => {
                        warn!("Failed to run ffmpeg to pre-generate thumbnail: {:?}", e);
                    }
                }
                // Xóa file thumbnail tạm
                let _ = tokio::fs::remove_file(&thumbnail_temp_path).await;
            } else {
                info!("ffmpeg not available, skipping local thumbnail pre-generation");
            }
            Ok(())
        }
        Err(e) => Err(e)
    }
}
