use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub username: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUser {
    pub username: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSession {
    pub id: i32,
    pub user_id: i32,
    pub letters: String,
    pub score: i32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewGameSession {
    pub user_id: i32,
    pub letters: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attempt {
    pub id: i32,
    pub session_id: i32,
    pub word: String,
    pub is_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAttempt {
    pub word: String,
}