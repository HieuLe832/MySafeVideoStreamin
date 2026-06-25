use crate::domain::ports::auth_verifier::AuthVerifier;
use serde::Deserialize;
use tracing::error;

pub struct GoogleAuthVerifier {
    google_client_id: String,
    http_client: reqwest::Client,
}

#[derive(Deserialize, Debug)]
struct GoogleTokenInfo {
    aud: String,
    email: String,
    email_verified: Option<serde_json::Value>,
}

impl GoogleAuthVerifier {
    pub fn new(google_client_id: String, http_client: reqwest::Client) -> Self {
        Self {
            google_client_id,
            http_client,
        }
    }
}

#[axum::async_trait]
impl AuthVerifier for GoogleAuthVerifier {
    async fn verify_token(&self, token: &str) -> Result<String, String> {
        let url = format!(
            "https://oauth2.googleapis.com/tokeninfo?id_token={}",
            urlencoding::encode(token)
        );

        let response = self.http_client.get(&url).send().await
            .map_err(|e| {
                error!("Error connecting to Google tokeninfo API: {:?}", e);
                format!("Không thể kết nối đến máy chủ Google Auth: {}", e)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Google tokeninfo verification failed. Status: {}, Body: {}", status, body);
            return Err("Mã xác thực Google không hợp lệ hoặc đã hết hạn.".into());
        }

        let info: GoogleTokenInfo = response.json().await
            .map_err(|e| {
                error!("Failed to parse Google tokeninfo response: {:?}", e);
                format!("Lỗi phân tích dữ liệu Google Auth: {}", e)
            })?;

        // Kiểm tra Audience (Client ID)
        if info.aud != self.google_client_id {
            error!("Client ID mismatch. Expected: {}, Got: {}", self.google_client_id, info.aud);
            return Err("Mã Client ID không trùng khớp.".into());
        }

        // Kiểm tra xem email đã được xác minh chưa
        let verified = match info.email_verified {
            Some(serde_json::Value::Bool(b)) => b,
            Some(serde_json::Value::String(s)) => s == "true",
            _ => false,
        };
        if !verified {
            error!("Google email is not verified: {}", info.email);
            return Err("Tài khoản Google chưa được xác minh.".into());
        }

        Ok(info.email)
    }
}
