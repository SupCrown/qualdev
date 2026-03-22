use axum::http::StatusCode;
use serde_json::Value;
use std::sync::OnceLock;

static HTTP: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    HTTP.get_or_init(|| reqwest::Client::new())
}

fn processing_base() -> String {
    std::env::var("PROCESSING_BASE").unwrap_or_else(|_| "http://127.0.0.1:3001".to_string())
}

/// Transfère la réponse HTTP + JSON du `processing_service`.
pub async fn proxy_post(path: &str, body: Value) -> Result<(StatusCode, Value), String> {
    let url = format!(
        "{}/{}",
        processing_base().trim_end_matches('/'),
        path.trim_start_matches('/')
    );
    let res = http_client()
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status =
        StatusCode::from_u16(res.status().as_u16()).map_err(|e| e.to_string())?;
    let val: Value = res
        .json()
        .await
        .map_err(|e| format!("JSON processing: {e}"))?;
    Ok((status, val))
}

pub async fn proxy_get(path: &str) -> Result<(StatusCode, Value), String> {
    let url = format!(
        "{}/{}",
        processing_base().trim_end_matches('/'),
        path.trim_start_matches('/')
    );
    let res = http_client().get(&url).send().await.map_err(|e| e.to_string())?;
    let status =
        StatusCode::from_u16(res.status().as_u16()).map_err(|e| e.to_string())?;
    let val: Value = res
        .json()
        .await
        .map_err(|e| format!("JSON processing: {e}"))?;
    Ok((status, val))
}
