use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct User {
    pub id: i32,
    pub username: String,
}

#[derive(Deserialize)]
pub struct NewUser {
    pub username: String,
}
