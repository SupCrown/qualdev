use axum::{Router, routing::{get, post}};

mod game;

pub fn router() -> Router {
    Router::new()
        .route("/api/game/start", post(game::game_start))
        .route("/api/game/state/{session_id}", get(game::game_state))
        .route("/api/guess", post(game::guess))
}
