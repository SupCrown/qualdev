//! Bibliothèque du **processing_service** : logique de jeu (feedback type Tusmo/Wordle)
//! isolée du binaire HTTP pour faciliter les tests unitaires et d’intégration légers.
//!
//! Le binaire (`main.rs`) ajoute la persistance PostgreSQL, gRPC vers `word_service` et les routes Axum.

pub mod game_logic;
