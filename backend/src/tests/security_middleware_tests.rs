use axum::{
    routing::get,
    Router,
    middleware::from_fn_with_state,
    response::IntoResponse,
    http::{Request, StatusCode},
};
use tower::util::ServiceExt;
use crate::presentation::web::middleware::security::security_headers_middleware;

async fn mock_handler() -> impl IntoResponse {
    StatusCode::OK
}

#[tokio::test]
async fn test_security_headers_injected() {
    let state = super::create_test_state();
    
    // Construct test router with security headers middleware
    let app = Router::new()
        .route("/", get(mock_handler))
        .route_layer(from_fn_with_state(state.clone(), security_headers_middleware))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let headers = response.headers();
    
    // Verify CSP header
    assert!(headers.contains_key("Content-Security-Policy"));
    assert_eq!(
        headers.get("Content-Security-Policy").unwrap(),
        "default-src 'self'; media-src 'self' blob: https://test-bucket.r2.cloudflarestorage.com;"
    );

    // Verify static security headers
    assert_eq!(headers.get("X-Content-Type-Options").unwrap(), "nosniff");
    assert_eq!(headers.get("X-Frame-Options").unwrap(), "DENY");
    assert_eq!(headers.get("Referrer-Policy").unwrap(), "strict-origin-when-cross-origin");
    assert_eq!(headers.get("Cross-Origin-Opener-Policy").unwrap(), "same-origin");
}
