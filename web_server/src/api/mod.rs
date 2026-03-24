use axum::{Router, routing::{get, post}};

mod game;

pub fn router() -> Router {
    Router::new()
        .route("/api/profile", post(game::post_profile))
        .route("/api/solo/start", post(game::post_solo_start))
        .route("/api/solo/state/{session_id}", get(game::get_solo_state))
        .route("/api/solo/guess", post(game::post_solo_guess))
        .route("/api/multi/host", post(game::post_multi_host))
        .route("/api/multi/join", post(game::post_multi_join))
        .route("/api/multi/state", get(game::get_multi_state))
        .route("/api/multi/guess", post(game::post_multi_guess))
}
