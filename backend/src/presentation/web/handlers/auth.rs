use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;
use std::sync::Arc;
use crate::presentation::web::router::PresentationState;

#[derive(Serialize)]
pub struct AuthConfigResponse {
    pub google_client_id: String,
}

// Handler cấu hình Auth (Public API)
pub async fn auth_config_handler(
    State(state): State<Arc<PresentationState>>,
) -> impl IntoResponse {
    Json(AuthConfigResponse {
        google_client_id: state.google_client_id.clone(),
    })
}
