use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::time::{Instant, Duration};
use crate::domain::ports::auth_verifier::AuthVerifier;

pub struct VerifyTokenUseCase {
    auth_verifier: Arc<dyn AuthVerifier>,
    token_cache: RwLock<HashMap<String, (Result<String, String>, Instant)>>,
    allowed_email: String,
}

impl VerifyTokenUseCase {
    pub fn new(auth_verifier: Arc<dyn AuthVerifier>, allowed_email: String) -> Self {
        Self {
            auth_verifier,
            token_cache: RwLock::new(HashMap::new()),
            allowed_email,
        }
    }

    pub async fn execute(&self, token: &str) -> Result<String, String> {
        // 1. Kiểm tra cache trong bộ nhớ trước (gồm cả hit thành công và hit thất bại)
        {
            let cache = self.token_cache.read().unwrap();
            if let Some((res, expires_at)) = cache.get(token) {
                if Instant::now() < *expires_at {
                    return res.clone();
                }
            }
        }

        // 2. Cache miss hoặc hết hạn -> Tiến hành xác thực qua AuthVerifier
        let verify_result = self.auth_verifier.verify_token(token).await;
        
        // 3. Phân quyền (chỉ cho phép email đã chỉ định)
        let final_result = match verify_result {
            Ok(email) => {
                if email.to_lowercase() == self.allowed_email.to_lowercase() {
                    Ok(email)
                } else {
                    Err(format!("Tài khoản Google ({}) không được phép truy cập ứng dụng này.", email))
                }
            }
            Err(e) => Err(e),
        };

        // 4. Ghi kết quả vào Cache
        {
            let mut cache = self.token_cache.write().unwrap();
            let now = Instant::now();
            
            // Dọn dẹp cache hết hạn
            cache.retain(|_, (_, expires_at)| now < *expires_at);

            // Phòng ngừa tràn RAM nếu bị spam ngẫu nhiên nhiều Token lỗi
            if cache.len() >= 1000 {
                cache.clear();
            }

            // Đặt thời gian sống: 10 phút cho thành công, 2 phút cho thất bại (Negative Caching)
            let ttl = if final_result.is_ok() {
                Duration::from_secs(600)
            } else {
                Duration::from_secs(120)
            };

            cache.insert(
                token.to_string(),
                (final_result.clone(), now + ttl),
            );
        }

        final_result
    }
}
