use axum::{Router, routing::get};
use dotenvy::dotenv;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use game_api::{db, routes}; // <-- IMPORTANT

#[tokio::main]
async fn main() {
    dotenv().ok();

    let pool = db::connect_db().await;

    let app = Router::new()
        .route("/health", get(|| async { "API OK" }))
        .nest("/api", routes::routes(pool));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🚀 API lancée sur http://{}", addr);

    let listener = TcpListener::bind(addr)
        .await
        .expect("Erreur bind adresse");

    axum::serve(listener, app)
        .await
        .expect("Erreur serveur");
}