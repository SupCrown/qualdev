//! Crate **processing_service** : ici il n’y a que `game_logic` (Wordle-like) testable sans réseau.
//! Le binaire `main.rs` empile par-dessus Axum, SQLx, tonic — classique séparation « domaine / infra ».

pub mod game_logic;
