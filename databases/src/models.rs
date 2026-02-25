use serde::{Deserialize, Serialize};

// --- Users ---

#[derive(Serialize)]
pub struct User {
    pub id: i32,
    pub username: String,
}

#[derive(Deserialize)]
pub struct NewUser {
    pub username: String,
}

// --- Game Sessions ---

#[derive(Serialize)]
pub struct GameSession {
    pub id: i32,
    pub user_id: i32,
    pub letters: String,
    pub score: i32,
    pub status: String,
}

#[derive(Deserialize)]
pub struct NewGameSession {
    pub user_id: i32,
    pub letters: String,
}

// --- Attempts ---

#[derive(Serialize)]
pub struct Attempt {
    pub id: i32,
    pub session_id: i32,
    pub word: String,
    pub is_valid: bool,
}

#[derive(Deserialize)]
pub struct NewAttempt {
    pub word: String,
}