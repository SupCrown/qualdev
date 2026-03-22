//! Serveur HTTP de logique de jeu Tusmo : sessions, tentatives, validation dictionnaire (gRPC),
//! persistance `game_db` (PostgreSQL).
//!
//! Voir [`processing_service::game_logic`] pour les règles de feedback et les tests associés.

mod db;

pub mod word {
    tonic::include_proto!("word");
}

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use db::{
    connect, get_or_create_user, get_session, insert_attempt, insert_session, list_attempts,
    update_session_progress, AttemptRow, SessionRow,
};
use processing_service::game_logic::{compute_feedback, is_win};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tonic::transport::Channel;
use word::word_service_client::WordServiceClient;
use word::{CheckWordRequest, GetRandomWordRequest, GetRandomWordResponse};

const MAX_ATTEMPTS: i32 = 5;
const TARGET_LEN_MIN: usize = 4;
const TARGET_LEN_MAX: usize = 8;

#[derive(Clone)]
struct AppState {
    pool: sqlx::PgPool,
    grpc: WordServiceClient<Channel>,
}

#[derive(Deserialize)]
struct StartRequest {
    username: Option<String>,
}

#[derive(Serialize)]
struct AttemptDto {
    word: String,
    feedback: Vec<String>,
}

#[derive(Serialize)]
struct StartResponse {
    player_username: String,
    session_id: i32,
    word_length: usize,
    max_attempts: u32,
    history: Vec<AttemptDto>,
    status: String,
    /// Première lettre du mot (indice au début de partie).
    #[serde(skip_serializing_if = "Option::is_none")]
    first_letter_hint: Option<String>,
}

#[derive(Deserialize)]
struct GuessRequest {
    session_id: i32,
    word: String,
}

#[derive(Serialize)]
struct GameStateBody {
    session_id: i32,
    word_length: usize,
    max_attempts: u32,
    attempts_used: u32,
    status: String,
    history: Vec<AttemptDto>,
    /// Mot secret révélé uniquement en cas de défaite (`status == "lost"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    target_word: Option<String>,
    /// Première lettre tant qu’aucune tentative n’a été jouée.
    #[serde(skip_serializing_if = "Option::is_none")]
    first_letter_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Mot aléatoire entre 4 et 8 lettres (selon word_service), en majuscules ASCII.
async fn fetch_random_target_word(
    client: &mut WordServiceClient<Channel>,
) -> Result<String, tonic::Status> {
    for _ in 0..128 {
        let response: tonic::Response<GetRandomWordResponse> =
            client.get_random_word(GetRandomWordRequest {}).await?;
        let w = response.into_inner().word;
        let normalized: String = w.chars().map(|c| c.to_ascii_uppercase()).collect();
        let n = normalized.chars().count();
        if (TARGET_LEN_MIN..=TARGET_LEN_MAX).contains(&n) {
            return Ok(normalized);
        }
    }
    Err(tonic::Status::resource_exhausted(
        "impossible d'obtenir un mot de 4 à 8 lettres depuis word_service",
    ))
}

fn row_to_dto(r: &AttemptRow) -> Result<AttemptDto, String> {
    let fb: Vec<String> = match &r.feedback {
        Some(j) => serde_json::from_value(j.0.clone()).map_err(|e| e.to_string())?,
        None => Vec::new(),
    };
    Ok(AttemptDto {
        word: r.word.clone(),
        feedback: fb,
    })
}

fn history_from_rows(rows: &[AttemptRow]) -> Result<Vec<AttemptDto>, String> {
    rows.iter().map(row_to_dto).collect()
}

async fn build_state(
    pool: &sqlx::PgPool,
    session: &SessionRow,
    err: Option<String>,
) -> Result<GameStateBody, String> {
    let rows = list_attempts(pool, session.id).await.map_err(|e| e.to_string())?;
    let history = history_from_rows(&rows)?;
    let target_word = (session.status == "lost").then(|| session.letters.clone());

    let first_letter_hint =
        (session.status == "active" && history.is_empty()).then(|| {
            session
                .letters
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_default()
        });
    let first_letter_hint = first_letter_hint.filter(|s| !s.is_empty());

    Ok(GameStateBody {
        session_id: session.id,
        word_length: session.letters.chars().count(),
        max_attempts: MAX_ATTEMPTS as u32,
        attempts_used: session.score as u32,
        status: session.status.clone(),
        history,
        target_word,
        first_letter_hint,
        error: err,
    })
}

async fn post_start(
    State(state): State<AppState>,
    Json(body): Json<StartRequest>,
) -> Result<Json<StartResponse>, (StatusCode, Json<Value>)> {
    let player_username = body
        .username
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("player_{}", uuid::Uuid::new_v4()));

    let user_id = get_or_create_user(&state.pool, &player_username)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;

    let mut client = state.grpc.clone();
    let target = fetch_random_target_word(&mut client).await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("word_service: {e}") })),
        )
    })?;

    let session_id = insert_session(&state.pool, user_id, &target)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;

    let first_letter_hint = target.chars().next().map(|c| c.to_string());

    Ok(Json(StartResponse {
        player_username,
        session_id,
        word_length: target.chars().count(),
        max_attempts: MAX_ATTEMPTS as u32,
        history: vec![],
        status: "active".to_string(),
        first_letter_hint,
    }))
}

async fn get_state(
    State(state): State<AppState>,
    Path(session_id): Path<i32>,
) -> Result<Json<GameStateBody>, (StatusCode, Json<Value>)> {
    let session = get_session(&state.pool, session_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "session introuvable" })),
        ))?;

    let body = build_state(&state.pool, &session, None)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;

    Ok(Json(body))
}

async fn post_guess(
    State(state): State<AppState>,
    Json(body): Json<GuessRequest>,
) -> Result<Json<GameStateBody>, (StatusCode, Json<Value>)> {
    let guess_raw = body.word.trim();
    if guess_raw.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "mot vide" })),
        ));
    }

    let session = get_session(&state.pool, body.session_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "session introuvable" })),
        ))?;

    if session.status != "active" {
        let msg = match session.status.as_str() {
            "won" => "partie déjà gagnée — lancez une nouvelle partie",
            "lost" => "partie déjà perdue — lancez une nouvelle partie",
            _ => "partie terminée",
        };
        return Err((StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))));
    }

    let target = &session.letters;
    let guess_upper: String = guess_raw.chars().map(|c| c.to_ascii_uppercase()).collect();

    if guess_upper.chars().count() != target.chars().count() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!(
                    "le mot doit faire {} lettres",
                    target.chars().count()
                )
            })),
        ));
    }

    let mut grpc = state.grpc.clone();
    let dict_ok = grpc
        .check_word(CheckWordRequest {
            word: guess_upper.clone(),
        })
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": format!("dictionnaire (word_service): {e}") })),
            )
        })?
        .into_inner()
        .is_valid;

    if !dict_ok {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "ce mot n'existe pas dans le dictionnaire",
                "code": "not_in_dictionary"
            })),
        ));
    }

    let feedback_vec = compute_feedback(&guess_upper, target).ok_or((
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "comparaison impossible" })),
    ))?;

    let feedback_json = serde_json::to_value(&feedback_vec).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;

    insert_attempt(
        &state.pool,
        session.id,
        &guess_upper,
        &feedback_json,
        true,
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;

    let new_score = session.score + 1;
    let won = is_win(&guess_upper, target);
    let lost = !won && new_score >= MAX_ATTEMPTS;

    let new_status = if won {
        "won"
    } else if lost {
        "lost"
    } else {
        "active"
    };

    update_session_progress(&state.pool, session.id, new_score, new_status)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;

    let updated = get_session(&state.pool, session.id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?
        .expect("session exists");

    let out = build_state(&state.pool, &updated, None)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;

    Ok(Json(out))
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let grpc_url = std::env::var("WORD_SERVICE_GRPC")
        .unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());

    let grpc = WordServiceClient::connect(grpc_url.clone())
        .await
        .unwrap_or_else(|e| {
            panic!("connexion word_service ({grpc_url}) impossible: {e}");
        });

    let pool = connect().await;

    let state = AppState {
        pool,
        grpc,
    };

    let addr =
        std::env::var("PROCESSING_BIND").unwrap_or_else(|_| "127.0.0.1:3001".to_string());

    let app = Router::new()
        .route("/api/game/start", post(post_start))
        .route("/api/game/state/{session_id}", get(get_state))
        .route("/api/guess", post(post_guess))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind");
    println!("processing_service → http://{addr}");
    println!("PostgreSQL via DATABASE_URL · word_service gRPC: {grpc_url}");

    axum::serve(listener, app).await.expect("serve");
}
