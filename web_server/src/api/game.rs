use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};

use crate::bridge::client;

pub async fn game_start(Json(payload): Json<Value>) -> impl IntoResponse {
    match client::proxy_post("/api/game/start", payload).await {
        Ok((status, body)) => (status, Json(body)).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}

pub async fn game_state(Path(session_id): Path<i32>) -> impl IntoResponse {
    let path = format!("/api/game/state/{session_id}");
    match client::proxy_get(&path).await {
        Ok((status, body)) => (status, Json(body)).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}

pub async fn guess(Json(payload): Json<Value>) -> impl IntoResponse {
    match client::proxy_post("/api/guess", payload).await {
        Ok((status, body)) => (status, Json(body)).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}
