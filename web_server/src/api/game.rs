use axum::Json;
use serde::{Deserialize, Serialize};
use crate::bridge::client;

#[derive(Deserialize)]
pub struct GuessRequest {
    pub word: String,
}

#[derive(Serialize)]
pub struct GuessResponse {
    pub message: String,
}

pub async fn guess(
    Json(payload): Json<GuessRequest>,
) -> Json<GuessResponse> {
    let response = client::send(&payload.word);

    Json(GuessResponse {
        message: response.message,
    })
}