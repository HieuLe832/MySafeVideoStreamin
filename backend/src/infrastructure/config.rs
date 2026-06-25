use tracing::{info, warn};

pub struct InfrastructureConfig {
    pub port: u16,
    pub allowed_origin_str: String,
    pub google_client_id: String,
    pub allowed_email: String,
    pub csp_header_value: String,
    pub r2_account_id: String,
    pub r2_access_key_id: String,
    pub r2_secret_access_key: String,
    pub r2_bucket_name: String,
}

impl InfrastructureConfig {
    pub fn load() -> Self {
        // Load các biến môi trường từ .env
        if let Err(e) = dotenvy::dotenv() {
            warn!("No .env file found, relying on environment variables: {}", e);
        }

        // Đọc cấu hình từ Env
        let r2_account_id = std::env::var("R2_ACCOUNT_ID")
            .expect("R2_ACCOUNT_ID is required in environment/env file");
        let r2_access_key_id = std::env::var("R2_ACCESS_KEY_ID")
            .expect("R2_ACCESS_KEY_ID is required in environment/env file");
        let r2_secret_access_key = std::env::var("R2_SECRET_ACCESS_KEY")
            .expect("R2_SECRET_ACCESS_KEY is required in environment/env file");
        let r2_bucket_name = std::env::var("R2_BUCKET_NAME")
            .expect("R2_BUCKET_NAME is required in environment/env file");
        
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .expect("PORT must be a valid number");

        // Đọc cấu hình Google Auth
        let google_client_id = std::env::var("GOOGLE_CLIENT_ID")
            .unwrap_or_else(|_| "YOUR_GOOGLE_CLIENT_ID.apps.googleusercontent.com".to_string());
        if google_client_id == "YOUR_GOOGLE_CLIENT_ID.apps.googleusercontent.com" || google_client_id.is_empty() {
            warn!("CẢNH BÁO: GOOGLE_CLIENT_ID chưa được cấu hình. Google Auth sẽ không thể xác thực thành công!");
        }

        let allowed_email = std::env::var("ALLOWED_EMAIL")
            .unwrap_or_else(|_| "admin@example.com".to_string());
        info!("Allowed account email: {}", allowed_email);

        // Đọc cấu hình bổ sung cho CSP
        let custom_csp_domains = std::env::var("CSP_ALLOWED_DOMAINS")
            .unwrap_or_else(|_| "".to_string());
        
        // Tự động thêm R2 Endpoint của user vào CSP (bao gồm cả endpoint trực tiếp và virtual-host bucket style)
        let mut allowed_domains = format!(
            "https://{}.r2.cloudflarestorage.com https://*.{}.r2.cloudflarestorage.com",
            r2_account_id, r2_account_id
        );
        if !custom_csp_domains.is_empty() {
            allowed_domains = format!("{} {}", allowed_domains, custom_csp_domains);
        }
        
        let csp_header_value = format!(
            "default-src 'self'; \
             script-src 'self' 'unsafe-inline' https://accounts.google.com https://apis.google.com; \
             style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
             font-src 'self' https://fonts.gstatic.com; \
             connect-src 'self' https://accounts.google.com https://oauth2.googleapis.com {}; \
             frame-src 'self' https://accounts.google.com; \
             img-src 'self' data: https://*.googleusercontent.com; \
             media-src 'self' blob: {}; \
             object-src 'none';",
            allowed_domains, allowed_domains
        );

        let allowed_origin_str = std::env::var("ALLOWED_ORIGIN").unwrap_or_else(|_| "*".to_string());

        Self {
            port,
            allowed_origin_str,
            google_client_id,
            allowed_email,
            csp_header_value,
            r2_account_id,
            r2_access_key_id,
            r2_secret_access_key,
            r2_bucket_name,
        }
    }
}
