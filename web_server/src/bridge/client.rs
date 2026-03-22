use serde::Deserialize;
use std::sync::OnceLock;

static HTTP: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    HTTP.get_or_init(|| reqwest::Client::new())
}

pub struct BackendResponse {
    pub message: String,
}

#[derive(Deserialize)]
struct ProcessingResponse {
    message: String,
}

/// Envoie la tentative au `processing_service` (HTTP POST JSON identique à `/api/guess`).
/// URL : variable d’environnement `PROCESSING_URL`, défaut `http://127.0.0.1:3001/api/guess`.
pub async fn send(word: &str) -> BackendResponse {
    let url = std::env::var("PROCESSING_URL").unwrap_or_else(|_| {
        "http://127.0.0.1:3001/api/guess".to_string()
    });
    let url_for_err = url.clone();

    let res = http_client()
        .post(&url)
        .json(&serde_json::json!({ "word": word }))
        .send()
        .await;

    match res {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                match resp.json::<ProcessingResponse>().await {
                    Ok(body) => BackendResponse {
                        message: body.message,
                    },
                    Err(e) => BackendResponse {
                        message: format!("Réponse du traitement illisible: {e}"),
                    },
                }
            } else {
                let text = resp.text().await.unwrap_or_default();
                BackendResponse {
                    message: format!("Erreur traitement ({status}): {text}"),
                }
            }
        }
        Err(e) => BackendResponse {
            message: format!(
                "Impossible de joindre le service de traitement ({url_for_err}): {e}"
            ),
        },
    }
}
