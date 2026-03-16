use processing_service::api_client::{ApiClient, ApiError};
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};
use serde_json::json;

#[tokio::test]
async fn get_game_session_404_returns_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/games/123"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let api = ApiClient::new(&format!("{}/api", server.uri()));
    let err = api.get_game_session(123).await.unwrap_err();

    match err {
        ApiError::NotFound => {}
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn create_user_returns_user() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/users"))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(json!({
                "id": 1,
                "username": "alice"
            }))
        )
        .mount(&server)
        .await;

    let api = ApiClient::new(&format!("{}/api", server.uri()));
    let user = api.create_user("alice").await.unwrap();

    assert_eq!(user.id, 1);
    assert_eq!(user.username, "alice");
}

#[tokio::test]
async fn submit_attempt_returns_attempt() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/games/10/attempt"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({
                "id": 99,
                "session_id": 10,
                "word": "SALON",
                "is_valid": true
            }))
        )
        .mount(&server)
        .await;

    let api = ApiClient::new(&format!("{}/api", server.uri()));
    let attempt = api.submit_attempt(10, "SALON").await.unwrap();

    assert_eq!(attempt.id, 99);
    assert_eq!(attempt.session_id, 10);
    assert_eq!(attempt.word, "SALON");
    assert!(attempt.is_valid);
}