//! API HTTP **game_api** (Axum + SQLx) sur le schéma PostgreSQL `game_db`.
//! Utilisée comme couche REST de démonstration ; le jeu Tusmo principal peut utiliser la BDD via `processing_service`.

pub mod db;
pub mod routes;
pub mod models;