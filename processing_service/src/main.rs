//! Binaire HTTP du jeu : profils, mode **solo** (6 tentatives / mot, série persistante),
//! mode **multi** (code = `id` PostgreSQL, course au 5ᵉ mot).
//! Le gros du calcul « Wordle » est dans `game_logic` ; ici c’est surtout de l’orchestration + SQL + gRPC.

mod db;
mod multi;
mod solo;

pub mod word {
    tonic::include_proto!("word");
}

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use db::{
    connect, get_or_create_user, get_user_profile, insert_party, migrate_best_score_column,
    migrate_parties_table, party_exists, top_leaderboard, update_best_score_if_better,
    LeaderRow,
};
use multi::{
    shared_parties, MultiWordsQuery, OtherScoreDto, PartyInner, PrivateAttemptDto, SharedParties,
    MULTI_WORDS_TO_WIN,
};
use processing_service::game_logic::{compute_feedback, is_win, LetterFeedback};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solo::{shared_solo, SoloAttemptDto, SoloSession, SharedSolo, SOLO_MAX_ATTEMPTS};
use tonic::transport::Channel;
use uuid::Uuid;
use word::word_service_client::WordServiceClient;
use word::{CheckWordRequest, GetRandomWordRequest, GetRandomWordResponse};

const TARGET_LEN_MIN: usize = 4;
const TARGET_LEN_MAX: usize = 8;

#[derive(Clone)]
struct AppState {
    pool: sqlx::PgPool,
    grpc: WordServiceClient<Channel>,
    solo: SharedSolo,
    parties: SharedParties,
}

#[derive(Deserialize)]
struct ProfileRequest {
    username: String,
}

#[derive(Serialize)]
struct ProfileResponse {
    user_id: i32,
    username: String,
    best_score: i32,
}

// ---- Solo ----

#[derive(Deserialize)]
struct SoloStartRequest {
    user_id: i32,
}

#[derive(Serialize)]
struct SoloStateResponse {
    session_id: String,
    word_length: usize,
    max_attempts: u32,
    attempts_used: u32,
    first_letter_hint: Option<String>,
    generation: u64,
    history: Vec<SoloAttemptDto>,
    current_streak: i32,
    leaderboard: Vec<LeaderRow>,
}

#[derive(Deserialize)]
struct SoloGuessRequest {
    session_id: String,
    user_id: i32,
    word: String,
}

#[derive(Serialize)]
struct SoloGuessResponse {
    won_word: bool,
    lost_word: bool,
    state: SoloStateResponse,
}

// ---- Multi ----

#[derive(Deserialize)]
struct MultiHostRequest {
    user_id: i32,
}

#[derive(Serialize)]
struct MultiHostResponse {
    party_id: i32,
}

#[derive(Deserialize)]
struct MultiJoinRequest {
    user_id: i32,
    party_id: i32,
}

#[derive(Serialize)]
struct MultiJoinResponse {
    party_id: i32,
}

#[derive(Deserialize)]
struct MultiGuessRequest {
    party_id: i32,
    user_id: i32,
    word: String,
}

#[derive(Serialize)]
struct MultiStateResponse {
    party_id: i32,
    word_length: usize,
    generation: u64,
    first_letter_hint: Option<String>,
    your_attempts: Vec<PrivateAttemptDto>,
    your_words_found: u8,
    others_scores: Vec<OtherScoreDto>,
    winner_user_id: Option<i32>,
    finished: bool,
}

#[derive(Serialize)]
struct MultiGuessResponse {
    you_found_word: bool,
    state: MultiStateResponse,
}

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

fn feedback_to_strings(f: &[LetterFeedback]) -> Vec<String> {
    f.iter()
        .map(|x| match x {
            LetterFeedback::Correct => "correct".to_string(),
            LetterFeedback::Present => "present".to_string(),
            LetterFeedback::Absent => "absent".to_string(),
        })
        .collect()
}

async fn solo_state_response(
    pool: &sqlx::PgPool,
    session_id: &str,
    s: &SoloSession,
) -> Result<SoloStateResponse, sqlx::Error> {
    let leaderboard = top_leaderboard(pool, 15).await?;
    Ok(SoloStateResponse {
        session_id: session_id.to_string(),
        word_length: s.target.chars().count(),
        max_attempts: SOLO_MAX_ATTEMPTS,
        attempts_used: s.attempts_this_word,
        first_letter_hint: if s.history.is_empty() {
            s.first_letter_hint()
        } else {
            None
        },
        generation: s.generation,
        history: s.history.clone(),
        current_streak: s.current_streak,
        leaderboard,
    })
}

async fn post_profile(
    State(state): State<AppState>,
    Json(body): Json<ProfileRequest>,
) -> Result<Json<ProfileResponse>, (StatusCode, Json<Value>)> {
    let username = body.username.trim();
    if username.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "nom vide" })),
        ));
    }
    let user_id = get_or_create_user(&state.pool, username)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;
    let row = get_user_profile(&state.pool, user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "profil introuvable" })),
        ))?;

    Ok(Json(ProfileResponse {
        user_id: row.id,
        username: row.username,
        best_score: row.best_score,
    }))
}

async fn post_solo_start(
    State(state): State<AppState>,
    Json(body): Json<SoloStartRequest>,
) -> Result<Json<SoloStateResponse>, (StatusCode, Json<Value>)> {
    get_user_profile(&state.pool, body.user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "joueur introuvable" })),
        ))?;

    let mut grpc = state.grpc.clone();
    let target = fetch_random_target_word(&mut grpc).await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("word_service: {e}") })),
        )
    })?;

    let session_id = Uuid::new_v4().to_string();
    let session = SoloSession {
        user_id: body.user_id,
        target,
        generation: 1,
        attempts_this_word: 0,
        history: Vec::new(),
        current_streak: 0,
    };

    let snap = solo_state_response(&state.pool, &session_id, &session)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;

    state.solo.lock().await.insert(session_id, session);

    Ok(Json(snap))
}

async fn get_solo_state(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<SoloStateResponse>, (StatusCode, Json<Value>)> {
    let solo = state.solo.lock().await;
    let s = solo.get(&session_id).ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "session inconnue" })),
    ))?;
    let body = solo_state_response(&state.pool, &session_id, s)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;
    Ok(Json(body))
}

async fn post_solo_guess(
    State(state): State<AppState>,
    Json(body): Json<SoloGuessRequest>,
) -> Result<Json<SoloGuessResponse>, (StatusCode, Json<Value>)> {
    let guess_raw = body.word.trim();
    if guess_raw.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "mot vide" })),
        ));
    }

    let guess_upper: String = guess_raw.chars().map(|c| c.to_ascii_uppercase()).collect();

    let mut won_word = false;
    let mut lost_word = false;

    let mut grpc = state.grpc.clone();

    {
        let mut map = state.solo.lock().await;
        let session = map.get_mut(&body.session_id).ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "session inconnue" })),
        ))?;

        if session.user_id != body.user_id {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "session d'un autre joueur" })),
            ));
        }

        if guess_upper.chars().count() != session.target.chars().count() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": format!(
                        "le mot doit faire {} lettres",
                        session.target.chars().count()
                    )
                })),
            ));
        }

        let dict_ok = grpc
            .check_word(CheckWordRequest {
                word: guess_upper.clone(),
            })
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({ "error": format!("dictionnaire: {e}") })),
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

        let feedback_vec = compute_feedback(&guess_upper, &session.target).ok_or((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "comparaison impossible" })),
        ))?;
        let fb_strings = feedback_to_strings(&feedback_vec);
        let win = is_win(&guess_upper, &session.target);

        session.history.push(SoloAttemptDto {
            word: guess_upper.clone(),
            feedback: fb_strings.clone(),
        });

        if win {
            won_word = true;
            session.current_streak += 1;
            update_best_score_if_better(&state.pool, body.user_id, session.current_streak)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": e.to_string() })),
                    )
                })?;

            let new_target = fetch_random_target_word(&mut grpc).await.map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({ "error": format!("word_service: {e}") })),
                )
            })?;
            session.target = new_target;
            session.generation += 1;
            session.attempts_this_word = 0;
            session.history.clear();
        } else {
            session.attempts_this_word += 1;
            if session.attempts_this_word >= SOLO_MAX_ATTEMPTS {
                lost_word = true;
                session.current_streak = 0;
                let new_target = fetch_random_target_word(&mut grpc).await.map_err(|e| {
                    (
                        StatusCode::BAD_GATEWAY,
                        Json(json!({ "error": format!("word_service: {e}") })),
                    )
                })?;
                session.target = new_target;
                session.generation += 1;
                session.attempts_this_word = 0;
                session.history.clear();
            }
        }
    }

    let session = state
        .solo
        .lock()
        .await
        .get(&body.session_id)
        .cloned()
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "session perdue" })),
        ))?;

    let snap = solo_state_response(&state.pool, &body.session_id, &session)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;

    Ok(Json(SoloGuessResponse {
        won_word,
        lost_word,
        state: snap,
    }))
}

async fn multi_state_response(
    pool: &sqlx::PgPool,
    party_id: i32,
    user_id: i32,
    p: &PartyInner,
) -> Result<MultiStateResponse, sqlx::Error> {
    let your_attempts = p
        .private_attempts
        .get(&user_id)
        .cloned()
        .unwrap_or_default();
    let your_words_found = *p.words_found.get(&user_id).unwrap_or(&0);

    let mut others_scores = Vec::new();
    for &uid in &p.participants {
        if uid == user_id {
            continue;
        }
        let row = get_user_profile(pool, uid).await?;
        let name = row.map(|r| r.username).unwrap_or_else(|| format!("user#{uid}"));
        others_scores.push(OtherScoreDto {
            user_id: uid,
            username: name,
            words_found: *p.words_found.get(&uid).unwrap_or(&0),
        });
    }
    others_scores.sort_by(|a, b| a.username.cmp(&b.username));

    Ok(MultiStateResponse {
        party_id,
        word_length: p.target.chars().count(),
        generation: p.generation,
        first_letter_hint: if your_attempts.is_empty() {
            p.first_letter_hint()
        } else {
            None
        },
        your_attempts,
        your_words_found,
        others_scores,
        winner_user_id: p.winner,
        finished: p.winner.is_some(),
    })
}

async fn post_multi_host(
    State(state): State<AppState>,
    Json(body): Json<MultiHostRequest>,
) -> Result<Json<MultiHostResponse>, (StatusCode, Json<Value>)> {
    get_user_profile(&state.pool, body.user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "joueur introuvable" })),
        ))?;

    let party_id = insert_party(&state.pool, body.user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;

    let mut grpc = state.grpc.clone();
    let target = fetch_random_target_word(&mut grpc).await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("word_service: {e}") })),
        )
    })?;

    let mut participants = std::collections::HashSet::new();
    participants.insert(body.user_id);

    let mut words_found = std::collections::HashMap::new();
    words_found.insert(body.user_id, 0);

    let inner = PartyInner {
        party_id,
        host_user_id: body.user_id,
        target,
        generation: 1,
        words_found,
        private_attempts: std::collections::HashMap::new(),
        participants,
        winner: None,
    };

    state.parties.lock().await.insert(party_id, inner);

    Ok(Json(MultiHostResponse { party_id }))
}

async fn post_multi_join(
    State(state): State<AppState>,
    Json(body): Json<MultiJoinRequest>,
) -> Result<Json<MultiJoinResponse>, (StatusCode, Json<Value>)> {
    if !party_exists(&state.pool, body.party_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?
    {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "code de partie invalide" })),
        ));
    }

    get_user_profile(&state.pool, body.user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "joueur introuvable" })),
        ))?;

    let mut map = state.parties.lock().await;
    let party = map.get_mut(&body.party_id).ok_or((
        StatusCode::GONE,
        Json(json!({
            "error": "partie non disponible (hôte absent ou serveur redémarré — recréez une partie)"
        })),
    ))?;

    if party.winner.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "partie terminée" })),
        ));
    }

    party.participants.insert(body.user_id);
    party.words_found.entry(body.user_id).or_insert(0);

    Ok(Json(MultiJoinResponse {
        party_id: body.party_id,
    }))
}

async fn get_multi_state(
    State(state): State<AppState>,
    Query(q): Query<MultiWordsQuery>,
) -> Result<Json<MultiStateResponse>, (StatusCode, Json<Value>)> {
    let map = state.parties.lock().await;
    let p = map.get(&q.party_id).ok_or((
        StatusCode::GONE,
        Json(json!({ "error": "partie non disponible" })),
    ))?;
    let body = multi_state_response(&state.pool, q.party_id, q.user_id, p)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;
    Ok(Json(body))
}

async fn post_multi_guess(
    State(state): State<AppState>,
    Json(body): Json<MultiGuessRequest>,
) -> Result<Json<MultiGuessResponse>, (StatusCode, Json<Value>)> {
    let guess_raw = body.word.trim();
    if guess_raw.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "mot vide" })),
        ));
    }

    let guess_upper: String = guess_raw.chars().map(|c| c.to_ascii_uppercase()).collect();
    let mut grpc = state.grpc.clone();
    let mut you_found_word = false;

    {
        let mut map = state.parties.lock().await;
        let party = map.get_mut(&body.party_id).ok_or((
            StatusCode::GONE,
            Json(json!({ "error": "partie non disponible" })),
        ))?;

        if party.winner.is_some() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "partie terminée" })),
            ));
        }

        if !party.participants.contains(&body.user_id) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "vous n'êtes pas dans cette partie" })),
            ));
        }

        if guess_upper.chars().count() != party.target.chars().count() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": format!(
                        "le mot doit faire {} lettres",
                        party.target.chars().count()
                    )
                })),
            ));
        }

        let dict_ok = grpc
            .check_word(CheckWordRequest {
                word: guess_upper.clone(),
            })
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({ "error": format!("dictionnaire: {e}") })),
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

        let feedback_vec = compute_feedback(&guess_upper, &party.target).ok_or((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "comparaison impossible" })),
        ))?;
        let fb_strings = feedback_to_strings(&feedback_vec);
        let win = is_win(&guess_upper, &party.target);

        party
            .private_attempts
            .entry(body.user_id)
            .or_default()
            .push(PrivateAttemptDto {
                word: guess_upper.clone(),
                feedback: fb_strings,
            });

        if win {
            you_found_word = true;
            let v = party.words_found.entry(body.user_id).or_insert(0);
            *v = v.saturating_add(1);
            if *v >= MULTI_WORDS_TO_WIN {
                party.winner = Some(body.user_id);
            }
            for uid in &party.participants {
                party.private_attempts.insert(*uid, Vec::new());
            }
            if party.winner.is_none() {
                let new_target = fetch_random_target_word(&mut grpc).await.map_err(|e| {
                    (
                        StatusCode::BAD_GATEWAY,
                        Json(json!({ "error": format!("word_service: {e}") })),
                    )
                })?;
                party.target = new_target;
                party.generation += 1;
            }
        }
    }

    let party = state
        .parties
        .lock()
        .await
        .get(&body.party_id)
        .cloned()
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "partie perdue" })),
        ))?;

    let snap = multi_state_response(&state.pool, body.party_id, body.user_id, &party)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;

    Ok(Json(MultiGuessResponse {
        you_found_word,
        state: snap,
    }))
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
    migrate_best_score_column(&pool)
        .await
        .expect("migration best_score");
    migrate_parties_table(&pool)
        .await
        .expect("migration parties");

    let state = AppState {
        pool,
        grpc,
        solo: shared_solo(),
        parties: shared_parties(),
    };

    let addr =
        std::env::var("PROCESSING_BIND").unwrap_or_else(|_| "127.0.0.1:3001".to_string());

    let app = Router::new()
        .route("/api/profile", post(post_profile))
        .route("/api/solo/start", post(post_solo_start))
        .route("/api/solo/state/{session_id}", get(get_solo_state))
        .route("/api/solo/guess", post(post_solo_guess))
        .route("/api/multi/host", post(post_multi_host))
        .route("/api/multi/join", post(post_multi_join))
        .route("/api/multi/state", get(get_multi_state))
        .route("/api/multi/guess", post(post_multi_guess))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind");
    println!("processing_service en ligne → http://{addr}");
    println!("BDD (DATABASE_URL) · gRPC mots : {grpc_url}");

    axum::serve(listener, app).await.expect("serve");
}
