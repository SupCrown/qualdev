use axum::{Router, routing::post};

mod game;

pub fn router() -> Router {
    Router::new()
        .route("/api/guess", post(game::guess))
}