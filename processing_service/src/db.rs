//! Couche SQLx sur `game_db` : users, parties multi, etc.
//! L’URL est pilotée par `DATABASE_URL` (en prod tu pointes vers ta vraie instance, en local souvent un `.env`).

use serde::Serialize;
use serde_json::Value;
use sqlx::{types::Json, PgPool};

/// Base distante Tusmo (surcharge possible avec la variable d’environnement `DATABASE_URL`).
const DEFAULT_DATABASE_URL: &str =
    "postgres://tusmo_admin:ton_mot_de_passe_super_secret@147.93.55.68/tusmo_db";

pub async fn connect() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&url)
        .await
        .expect("connexion PostgreSQL impossible")
}

/// Ajoute `best_score` si absent (meilleure **série** de mots solo trouvés d’affilée sans échec).
pub async fn migrate_best_score_column(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        ALTER TABLE game_db.users
        ADD COLUMN IF NOT EXISTS best_score INTEGER NOT NULL DEFAULT 0
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Table des parties multijoueur (l’`id` sert de **code** pour rejoindre).
pub async fn migrate_parties_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS game_db.parties (
            id SERIAL PRIMARY KEY,
            host_user_id INTEGER NOT NULL REFERENCES game_db.users(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_party(pool: &PgPool, host_user_id: i32) -> Result<i32, sqlx::Error> {
    let id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO game_db.parties (host_user_id)
        VALUES ($1)
        RETURNING id
        "#,
    )
    .bind(host_user_id)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn party_exists(pool: &PgPool, party_id: i32) -> Result<bool, sqlx::Error> {
    let n: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::bigint FROM game_db.parties WHERE id = $1"#,
    )
    .bind(party_id)
    .fetch_one(pool)
    .await?;
    Ok(n > 0)
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

#[derive(Debug, sqlx::FromRow)]
pub struct UserProfileRow {
    pub id: i32,
    pub username: String,
    pub best_score: i32,
}

pub async fn get_user_profile(pool: &PgPool, user_id: i32) -> Result<Option<UserProfileRow>, sqlx::Error> {
    sqlx::query_as::<_, UserProfileRow>(
        r#"
        SELECT id, username, COALESCE(best_score, 0)::integer AS best_score
        FROM game_db.users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_profile_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<UserProfileRow>, sqlx::Error> {
    sqlx::query_as::<_, UserProfileRow>(
        r#"
        SELECT id, username, COALESCE(best_score, 0)::integer AS best_score
        FROM game_db.users
        WHERE username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

/// Met à jour le record (meilleure série solo) uniquement si la nouvelle série est **strictement meilleure**.
pub async fn update_best_score_if_better(
    pool: &PgPool,
    user_id: i32,
    new_cycle_total: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE game_db.users
        SET best_score = $1
        WHERE id = $2
          AND $1 > COALESCE(best_score, 0)
        "#,
    )
    .bind(new_cycle_total)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct LeaderRow {
    pub username: String,
    pub best_score: i32,
}

pub async fn top_leaderboard(pool: &PgPool, limit: i64) -> Result<Vec<LeaderRow>, sqlx::Error> {
    sqlx::query_as::<_, LeaderRow>(
        r#"
        SELECT username, COALESCE(best_score, 0)::integer AS best_score
        FROM game_db.users
        ORDER BY best_score DESC, username ASC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

// ---- Anciennes tables (sessions solo) — conservées pour compat schéma mais non utilisées par la salle ----

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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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
