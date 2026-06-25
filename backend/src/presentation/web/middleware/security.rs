use axum::{
    extract::{State, Request},
    response::Response,
    middleware::Next,
};
use std::sync::Arc;
use crate::presentation::web::router::PresentationState;

// Middleware tiêm Security Headers cho toàn bộ phản hồi HTTP
pub async fn security_headers_middleware(
    State(state): State<Arc<PresentationState>>,
    req: Request,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    
    // Content-Security-Policy động từ PresentationState
    if let Ok(csp_val) = state.csp_header_value.parse() {
        headers.insert("Content-Security-Policy", csp_val);
    }
    
    // Các header bảo mật tĩnh khác
    if let Ok(nosniff_val) = "nosniff".parse() {
        headers.insert("X-Content-Type-Options", nosniff_val);
    }
    if let Ok(deny_val) = "DENY".parse() {
        headers.insert("X-Frame-Options", deny_val);
    }
    if let Ok(referrer_val) = "strict-origin-when-cross-origin".parse() {
        headers.insert("Referrer-Policy", referrer_val);
    }
    if let Ok(coop_val) = "same-origin".parse() {
        headers.insert("Cross-Origin-Opener-Policy", coop_val);
    }
    
    response
}
