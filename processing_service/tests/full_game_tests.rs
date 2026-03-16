use processing_service::api_client::ApiClient;
use serde_json::json;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn full_game_flow_with_finish() {
    let server = MockServer::start().await;
    let api = ApiClient::new(&format!("{}/api", server.uri()));

    Mock::given(method("POST"))
        .and(path("/api/users"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "id": 1,
            "username": "alice"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/games"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "id": 10,
            "user_id": 1,
            "letters": "salon",
            "score": 0,
            "status": "in_progress"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/games/10/attempt"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 100,
            "session_id": 10,
            "word": "SALON",
            "is_valid": true
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/games/10/finish"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 10,
            "user_id": 1,
            "letters": "salon",
            "score": 5,
            "status": "finished"
        })))
        .mount(&server)
        .await;

    let user = api.create_user("alice").await.unwrap();
    let session = api.create_game_session(user.id, "salon").await.unwrap();
    let attempt = api.submit_attempt(session.id, "SALON").await.unwrap();
    assert!(attempt.is_valid);

    let finished = api.finish_game_session(session.id).await.unwrap();
    assert_eq!(finished.status, "finished");
}