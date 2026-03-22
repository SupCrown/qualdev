use axum::{
    Router,
    routing::{get, post},
    extract::{State, Path},
    Json,
    http::StatusCode,
    response::IntoResponse,
};
use sqlx::PgPool;

use crate::models::{User, NewUser, GameSession, NewGameSession, Attempt, NewAttempt};

pub fn routes(pool: PgPool) -> Router {
    Router::new()
        // Users
        .route("/users", post(create_user))
        .route("/users", get(list_users))
        .route("/users/:id/scores", get(get_user_scores))
        // Game Sessions
        .route("/games", post(create_game_session))
        .route("/games/:id", get(get_game_session))
        // Attempts
        .route("/games/:id/attempt", post(submit_attempt))
        .route("/games/:id/finish", post(finish_game_session))
        .with_state(pool)
}

// ─── Users ────────────────────────────────────────────────────────────────────

async fn create_user(
    State(pool): State<PgPool>,
    Json(payload): Json<NewUser>,
) -> Result<(StatusCode, Json<User>), (StatusCode, String)> {
    let user = sqlx::query_as!(
        User,
        r#"INSERT INTO game_db.users (username) VALUES ($1) RETURNING id, username"#,
        payload.username
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(user)))
}

async fn list_users(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<User>>, (StatusCode, String)> {
    let users = sqlx::query_as!(
        User,
        r#"SELECT id, username FROM game_db.users"#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(users))
}

/// Retourne toutes les sessions d'un utilisateur avec son score total
async fn get_user_scores(
    State(pool): State<PgPool>,
    Path(user_id): Path<i32>,
) -> Result<Json<Vec<GameSession>>, (StatusCode, String)> {
    let sessions = sqlx::query_as!(
        GameSession,
        r#"SELECT id, user_id, letters, score, status FROM game_db.game_sessions WHERE user_id = $1 ORDER BY id DESC"#,
        user_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(sessions))
}

// ─── Game Sessions ─────────────────────────────────────────────────────────────

async fn create_game_session(
    State(pool): State<PgPool>,
    Json(payload): Json<NewGameSession>,
) -> Result<(StatusCode, Json<GameSession>), (StatusCode, String)> {
    let session = sqlx::query_as!(
        GameSession,
        r#"
        INSERT INTO game_db.game_sessions (user_id, letters)
        VALUES ($1, $2)
        RETURNING id, user_id, letters, score, status
        "#,
        payload.user_id,
        payload.letters
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(session)))
}

async fn get_game_session(
    State(pool): State<PgPool>,
    Path(session_id): Path<i32>,
) -> Result<Json<GameSession>, (StatusCode, String)> {
    let session = sqlx::query_as!(
        GameSession,
        r#"SELECT id, user_id, letters, score, status FROM game_db.game_sessions WHERE id = $1"#,
        session_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Session introuvable".to_string()))?;

    Ok(Json(session))
}

// ─── Attempts ─────────────────────────────────────────────────────────────────

/// Soumet un mot pour une session donnée.
/// La validation (est-ce que les lettres sont utilisables, est-ce un vrai mot)
/// est faite ici côté API. Si le mot est valide, le score de la session est incrémenté.
async fn submit_attempt(
    State(pool): State<PgPool>,
    Path(session_id): Path<i32>,
    Json(payload): Json<NewAttempt>,
) -> Result<Json<Attempt>, (StatusCode, String)> {
    // Récupère la session
    let session = sqlx::query_as!(
        GameSession,
        r#"SELECT id, user_id, letters, score, status FROM game_db.game_sessions WHERE id = $1"#,
        session_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Session introuvable".to_string()))?;

    if session.status == "finished" {
        return Err((StatusCode::BAD_REQUEST, "La session est terminée".to_string()));
    }

    // Vérifie que le mot peut être formé à partir des lettres disponibles
    let is_valid = word_can_be_formed(&payload.word, &session.letters);

    // Enregistre la tentative
    let attempt = sqlx::query_as!(
        Attempt,
        r#"
        INSERT INTO game_db.attempts (session_id, word, is_valid)
        VALUES ($1, $2, $3)
        RETURNING id, session_id, word, is_valid
        "#,
        session_id,
        payload.word,
        is_valid
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Si le mot est valide, on ajoute sa longueur au score
    if is_valid {
        sqlx::query!(
            r#"UPDATE game_db.game_sessions SET score = score + $1 WHERE id = $2"#,
            payload.word.len() as i32,
            session_id
        )
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(Json(attempt))
}

// ─── Helper ────────────────────────────────────────────────────────────────────

/// Vérifie que chaque lettre du mot proposé est disponible dans les lettres de la session
fn word_can_be_formed(word: &str, letters: &str) -> bool {
    let mut available: Vec<char> = letters.to_lowercase().chars().collect();

    for ch in word.to_lowercase().chars() {
        if let Some(pos) = available.iter().position(|&c| c == ch) {
            available.remove(pos);
        } else {
            return false;
        }
    }
    true
}

async fn finish_game_session(
    State(pool): State<PgPool>,
    Path(session_id): Path<i32>,
) -> Result<Json<GameSession>, (StatusCode, String)> {
    let session = sqlx::query_as!(
        GameSession,
        r#"
        UPDATE game_db.game_sessions
        SET status = 'finished'
        WHERE id = $1
        RETURNING id, user_id, letters, score, status
        "#,
        session_id
    )
        .fetch_optional(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Session introuvable".to_string()))?;

    Ok(Json(session))
}