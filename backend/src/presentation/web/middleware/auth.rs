use axum::{
    extract::{State, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
    middleware::Next,
};
use std::sync::Arc;
use crate::presentation::web::router::PresentationState;
use crate::presentation::web::handlers::ErrorResponse;

// Axum Middleware xác thực Google ID Token
pub async fn auth_middleware(
    State(state): State<Arc<PresentationState>>,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    let auth_header = req.headers().get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token_from_header = auth_header.and_then(|auth| {
        if auth.starts_with("Bearer ") {
            Some(auth[7..].to_string())
        } else {
            None
        }
    });

    let token = match token_from_header {
        Some(t) => t,
        None => {
            // Thử lấy token từ query parameter (ví dụ: ?token=...) dùng cho thẻ <img> load thumbnail
            let query_token = req.uri().query()
                .and_then(|q| {
                    q.split('&')
                        .find(|part| part.starts_with("token="))
                        .map(|part| {
                            let raw_val = &part["token=".len()..];
                            urlencoding::decode(raw_val)
                                .map(|cow| cow.into_owned())
                                .unwrap_or_else(|_| raw_val.to_string())
                        })
                });

            match query_token {
                Some(t) => t,
                None => {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        Json(ErrorResponse {
                            error: "Yêu cầu mã xác thực Google Auth (Bearer Token hoặc token query parameter).".into(),
                        }),
                    ).into_response());
                }
            }
        }
    };

    match state.verify_token_use_case.execute(&token).await {
        Ok(_) => {
            let response = next.run(req).await;
            Ok(response)
        }
        Err(err_msg) => {
            Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: err_msg,
                }),
            ).into_response())
        }
    }
}
