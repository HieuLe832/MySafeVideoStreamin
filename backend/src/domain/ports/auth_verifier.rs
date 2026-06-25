#[axum::async_trait]
pub trait AuthVerifier: Send + Sync {
    async fn verify_token(&self, token: &str) -> Result<String, String>;
}
