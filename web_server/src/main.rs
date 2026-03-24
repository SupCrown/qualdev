//! Front tout simple : on sert le statique dans `static/`, et on **proxy** tout `/api/*`
//! vers le `processing_service` (URL via `PROCESSING_BASE`). Rien de magique : du Axum + reqwest.

use axum::Router;
use tower_http::services::ServeDir;

mod api;
mod bridge;

#[tokio::main]
async fn main() {
    let bind_addr =
        std::env::var("WEB_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    let app = Router::new()
        .merge(api::router())
        .nest_service("/", ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap();

    println!("web_server → http://{bind_addr}");
    println!(
        "Proxy API vers PROCESSING_BASE (défaut http://127.0.0.1:3001)"
    );

    axum::serve(listener, app).await.unwrap();
}