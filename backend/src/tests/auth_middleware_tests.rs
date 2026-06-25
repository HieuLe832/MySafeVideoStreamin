use axum::{
    routing::get,
    Router,
    middleware::from_fn_with_state,
    response::IntoResponse,
    http::{Request, StatusCode, header},
};
use tower::util::ServiceExt;
use crate::presentation::web::middleware::auth::auth_middleware;

async fn mock_handler() -> impl IntoResponse {
    StatusCode::OK
}

#[tokio::test]
async fn test_auth_middleware_missing_header() {
    let state = super::create_test_state();
    let app = Router::new()
        .route("/", get(mock_handler))
        .route_layer(from_fn_with_state(state.clone(), auth_middleware))
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

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_invalid_prefix() {
    let state = super::create_test_state();
    let app = Router::new()
        .route("/", get(mock_handler))
        .route_layer(from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header(header::AUTHORIZATION, "Basic abcdef")
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_invalid_token() {
    let state = super::create_test_state();
    let app = Router::new()
        .route("/", get(mock_handler))
        .route_layer(from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header(header::AUTHORIZATION, "Bearer invalid-token")
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_success() {
    let state = super::create_test_state();
    let app = Router::new()
        .route("/", get(mock_handler))
        .route_layer(from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header(header::AUTHORIZATION, "Bearer valid-token")
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
