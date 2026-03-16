use reqwest::StatusCode;

use crate::models::{
    Attempt, GameSession, NewAttempt, NewGameSession, NewUser, User,
};

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    NotFound,
    BadRequest(String),
    Http(reqwest::Error),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NotFound => write!(f, "NOT_FOUND"),
            ApiError::BadRequest(msg) => write!(f, "BAD_REQUEST: {msg}"),
            ApiError::Http(e) => write!(f, "HTTP error: {e}"),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        ApiError::Http(e)
    }
}

#[derive(Clone, Debug)]
pub struct ApiClient {
    base_url: String,
    client: reqwest::Client,
}

impl ApiClient {
    /// base_url exemple: "http://localhost:3000/api"
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    // ---- Users ----

    pub async fn create_user(&self, username: &str) -> ApiResult<User> {
        let resp = self
            .client
            .post(self.url("/users"))
            .json(&NewUser {
                username: username.to_string(),
            })
            .send()
            .await?;

        if resp.status() == StatusCode::BAD_REQUEST {
            let msg = resp.text().await.unwrap_or_default();
            return Err(ApiError::BadRequest(msg));
        }

        Ok(resp.error_for_status()?.json().await?)
    }

    pub async fn list_users(&self) -> ApiResult<Vec<User>> {
        let resp = self.client.get(self.url("/users")).send().await?;
        Ok(resp.error_for_status()?.json().await?)
    }

    pub async fn get_user_scores(&self, user_id: i32) -> ApiResult<Vec<GameSession>> {
        let resp = self
            .client
            .get(self.url(&format!("/users/{}/scores", user_id)))
            .send()
            .await?;

        if resp.status() == StatusCode::NOT_FOUND {
            return Err(ApiError::NotFound);
        }

        Ok(resp.error_for_status()?.json().await?)
    }

    // ---- Games ----

    pub async fn create_game_session(&self, user_id: i32, letters: &str) -> ApiResult<GameSession> {
        let resp = self
            .client
            .post(self.url("/games"))
            .json(&NewGameSession {
                user_id,
                letters: letters.to_string(),
            })
            .send()
            .await?;

        if resp.status() == StatusCode::BAD_REQUEST {
            let msg = resp.text().await.unwrap_or_default();
            return Err(ApiError::BadRequest(msg));
        }

        Ok(resp.error_for_status()?.json().await?)
    }

    pub async fn get_game_session(&self, session_id: i32) -> ApiResult<GameSession> {
        let resp = self
            .client
            .get(self.url(&format!("/games/{}", session_id)))
            .send()
            .await?;

        if resp.status() == StatusCode::NOT_FOUND {
            return Err(ApiError::NotFound);
        }

        Ok(resp.error_for_status()?.json().await?)
    }

    // ---- Attempts (ton "guess") ----

    pub async fn submit_attempt(&self, session_id: i32, word: &str) -> ApiResult<Attempt> {
        let resp = self
            .client
            .post(self.url(&format!("/games/{}/attempt", session_id)))
            .json(&NewAttempt {
                word: word.to_string(),
            })
            .send()
            .await?;

        match resp.status() {
            StatusCode::NOT_FOUND => return Err(ApiError::NotFound),
            StatusCode::BAD_REQUEST => {
                let msg = resp.text().await.unwrap_or_default();
                return Err(ApiError::BadRequest(msg));
            }
            _ => {}
        }

        Ok(resp.error_for_status()?.json().await?)
    }

    pub async fn finish_game_session(&self, session_id: i32) -> ApiResult<GameSession> {
        let url = self.url(&format!("/games/{}/finish", session_id));

        println!("[HTTP->API] POST {} body=<empty>", url);

        let resp = self.client
            .post(&url)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        println!("[API->HTTP] status={} body={}", status, text);

        match status {
            reqwest::StatusCode::NOT_FOUND => return Err(ApiError::NotFound),
            reqwest::StatusCode::BAD_REQUEST => return Err(ApiError::BadRequest(text)),
            _ => {}
        }

        let session: GameSession = serde_json::from_str(&text)
            .map_err(|e| ApiError::BadRequest(format!("JSON parse error: {e}")))?;

        Ok(session)
    }
}