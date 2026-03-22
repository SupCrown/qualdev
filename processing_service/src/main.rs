use axum::{
    Json, Router,
    extract::State,
    routing::post,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Mot secret à deviner (remplacé plus tard par l’API / BDD).
/// Surcharge possible : `TUSMO_TARGET_WORD=CHIEN cargo run`
fn target_word() -> String {
    std::env::var("TUSMO_TARGET_WORD").unwrap_or_else(|_| "TUSMO".to_string())
}

/// Pour chaque position : la lettre si elle correspond au mot cible, sinon `-`.
fn mask_guess(guess: &str, target: &str) -> Result<String, &'static str> {
    let g: Vec<char> = guess.chars().collect();
    let t: Vec<char> = target.chars().collect();
    if g.len() != t.len() {
        return Err("longueur différente du mot cible");
    }
    let mut out = String::with_capacity(g.len() * 2); // lettre + éventuellement tiret entre
    for (i, (&gc, &tc)) in g.iter().zip(t.iter()).enumerate() {
        if i > 0 {
            out.push('-');
        }
        if gc.eq_ignore_ascii_case(&tc) {
            out.push(tc.to_ascii_uppercase());
        } else {
            out.push('-');
        }
    }
    Ok(out)
}

#[derive(Clone)]
struct AppState {
    target: Arc<str>,
}

#[derive(Deserialize)]
struct GuessRequest {
    word: String,
}

#[derive(Serialize)]
struct GuessResponse {
    message: String,
}

async fn guess(
    State(state): State<AppState>,
    Json(payload): Json<GuessRequest>,
) -> Json<GuessResponse> {
    let message = match mask_guess(&payload.word, &state.target) {
        Ok(masked) => masked,
        Err(e) => format!("erreur: {e}"),
    };
    Json(GuessResponse { message })
}

#[tokio::main]
async fn main() {
    let target = target_word();
    let addr = std::env::var("PROCESSING_BIND").unwrap_or_else(|_| "127.0.0.1:3001".to_string());

    let state = AppState {
        target: Arc::from(target.into_boxed_str()),
    };
    let target_len = state.target.len();

    let app = Router::new()
        .route("/api/guess", post(guess))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind processing_service");
    println!("processing_service écoute sur http://{addr} (POST /api/guess)");
    println!("mot cible (longueur {target_len}) — variable TUSMO_TARGET_WORD pour le changer");

    axum::serve(listener, app).await.expect("serve");
}

#[cfg(test)]
mod tests {
    use super::mask_guess;

    #[test]
    fn mask_matches_positions() {
        assert_eq!(mask_guess("tusmx", "tusmo").unwrap(), "T-U-S-M--");
    }

    #[test]
    fn wrong_length_errors() {
        assert!(mask_guess("ab", "abc").is_err());
    }
}
