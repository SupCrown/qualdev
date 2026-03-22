use axum::Router;
use tower_http::services::ServeDir;

mod api;
mod bridge;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .merge(api::router())
        .nest_service("/", ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("Serveur lancé sur http://127.0.0.1:3000");
    println!(
        "Processing (proxy) : PROCESSING_BASE ou défaut http://127.0.0.1:3001"
    );

    axum::serve(listener, app).await.unwrap();
}