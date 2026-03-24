use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};

use crate::bridge::client;

pub async fn post_profile(Json(payload): Json<Value>) -> impl IntoResponse {
    match client::proxy_post("/api/profile", payload).await {
        Ok((status, body)) => (status, Json(body)).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}

pub async fn post_solo_start(Json(payload): Json<Value>) -> impl IntoResponse {
    match client::proxy_post("/api/solo/start", payload).await {
        Ok((status, body)) => (status, Json(body)).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}

pub async fn get_solo_state(Path(session_id): Path<String>) -> impl IntoResponse {
    let path = format!("/api/solo/state/{session_id}");
    match client::proxy_get(&path).await {
        Ok((status, body)) => (status, Json(body)).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}

pub async fn post_solo_guess(Json(payload): Json<Value>) -> impl IntoResponse {
    match client::proxy_post("/api/solo/guess", payload).await {
        Ok((status, body)) => (status, Json(body)).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}

pub async fn post_multi_host(Json(payload): Json<Value>) -> impl IntoResponse {
    match client::proxy_post("/api/multi/host", payload).await {
        Ok((status, body)) => (status, Json(body)).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}

pub async fn post_multi_join(Json(payload): Json<Value>) -> impl IntoResponse {
    match client::proxy_post("/api/multi/join", payload).await {
        Ok((status, body)) => (status, Json(body)).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}

pub async fn get_multi_state(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let party = params.get("party_id").cloned().unwrap_or_default();
    let user = params.get("user_id").cloned().unwrap_or_default();
    let path = format!("/api/multi/state?party_id={}&user_id={}", party, user);
    match client::proxy_get(&path).await {
        Ok((status, body)) => (status, Json(body)).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}

pub async fn post_multi_guess(Json(payload): Json<Value>) -> impl IntoResponse {
    match client::proxy_post("/api/multi/guess", payload).await {
        Ok((status, body)) => (status, Json(body)).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}
