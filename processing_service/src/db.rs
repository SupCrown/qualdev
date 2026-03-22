//! Accès PostgreSQL au schéma `game_db` (utilisateurs, sessions, tentatives).
//!
//! L’URL par défaut peut être surchargée par la variable d’environnement `DATABASE_URL`.

use serde_json::Value;
use sqlx::{types::Json, PgPool};

/// Base distante Tusmo (surcharge possible avec la variable d’environnement `DATABASE_URL`).
const DEFAULT_DATABASE_URL: &str =
    "postgres://tusmo_admin:ton_mot_de_passe_super_secret@147.93.55.68/tusmo_db";

pub async fn connect() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("connexion PostgreSQL impossible")
}

pub async fn get_or_create_user(pool: &PgPool, username: &str) -> Result<i32, sqlx::Error> {
    let found: Option<i32> = sqlx::query_scalar(
        r#"SELECT id FROM game_db.users WHERE username = $1"#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    if let Some(id) = found {
        return Ok(id);
    }

    let id: i32 = sqlx::query_scalar(
        r#"INSERT INTO game_db.users (username) VALUES ($1) RETURNING id"#,
    )
    .bind(username)
    .fetch_one(pool)
    .await?;

    Ok(id)
}

pub async fn insert_session(pool: &PgPool, user_id: i32, target_word: &str) -> Result<i32, sqlx::Error> {
    let id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO game_db.game_sessions (user_id, letters, score, status)
        VALUES ($1, $2, 0, 'active')
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(target_word)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

#[derive(Debug, sqlx::FromRow)]
pub struct SessionRow {
    pub id: i32,
    #[allow(dead_code)]
    pub user_id: i32,
    pub letters: String,
    pub score: i32,
    pub status: String,
}

pub async fn get_session(pool: &PgPool, session_id: i32) -> Result<Option<SessionRow>, sqlx::Error> {
    sqlx::query_as::<_, SessionRow>(
        r#"SELECT id, user_id, letters, score, status FROM game_db.game_sessions WHERE id = $1"#,
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct AttemptRow {
    pub word: String,
    pub feedback: Option<Json<Value>>,
}

pub async fn list_attempts(pool: &PgPool, session_id: i32) -> Result<Vec<AttemptRow>, sqlx::Error> {
    sqlx::query_as::<_, AttemptRow>(
        r#"
        SELECT word, feedback
        FROM game_db.attempts
        WHERE session_id = $1
        ORDER BY id ASC
        "#,
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
}

pub async fn insert_attempt(
    pool: &PgPool,
    session_id: i32,
    word: &str,
    feedback: &Value,
    is_valid: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO game_db.attempts (session_id, word, is_valid, feedback)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(session_id)
    .bind(word)
    .bind(is_valid)
    .bind(feedback)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_session_progress(
    pool: &PgPool,
    session_id: i32,
    new_score: i32,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE game_db.game_sessions SET score = $1, status = $2 WHERE id = $3"#,
    )
    .bind(new_score)
    .bind(status)
    .bind(session_id)
    .execute(pool)
    .await?;
    Ok(())
}
