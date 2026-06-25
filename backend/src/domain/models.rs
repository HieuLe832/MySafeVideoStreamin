use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VideoInfo {
    pub key: String,
    pub original_name: String,
    pub size: u64,
    pub uploaded_at: String,
    pub stream_url: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ActiveDownload {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub status: String, // "Downloading" | "Uploading" | "Failed"
    pub error: Option<String>,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}
