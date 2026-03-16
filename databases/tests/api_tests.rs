use axum::{
    Router,
    routing::get,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt; // oneshot
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

async fn app() -> Router {
    dotenvy::dotenv().ok();
    let pool = game_api::db::connect_db().await;

    Router::new()
        .route("/health", get(|| async { "API OK" }))
        .nest("/api", game_api::routes::routes(pool))
}

#[tokio::test]
async fn health_ok() {
    let app = app().await;

    let res = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn create_user_then_list_users() {
    let app = app().await;

    let unique_username = format!(
        "test_user_{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let req = Request::builder()
        .method("POST")
        .uri("/api/users")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({ "username": unique_username }).to_string()
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    let users = v.as_array().unwrap();
    assert!(users.iter().any(|u| u["username"] == unique_username));
}

#[tokio::test]
async fn finish_game_session_works() {
    let app = app().await;

    let unique_username = format!(
        "finish_user_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    // 1) create user
    let req_user = Request::builder()
        .method("POST")
        .uri("/api/users")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({ "username": unique_username }).to_string()
        ))
        .unwrap();

    let res_user = app.clone().oneshot(req_user).await.unwrap();
    assert_eq!(res_user.status(), StatusCode::CREATED);

    let user_bytes = to_bytes(res_user.into_body(), usize::MAX).await.unwrap();
    let user_json: serde_json::Value = serde_json::from_slice(&user_bytes).unwrap();
    let user_id = user_json["id"].as_i64().unwrap() as i32;

    // 2) create game
    let req_game = Request::builder()
        .method("POST")
        .uri("/api/games")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "user_id": user_id,
                "letters": "salon"
            })
                .to_string()
        ))
        .unwrap();

    let res_game = app.clone().oneshot(req_game).await.unwrap();
    assert_eq!(res_game.status(), StatusCode::CREATED);

    let game_bytes = to_bytes(res_game.into_body(), usize::MAX).await.unwrap();
    let game_json: serde_json::Value = serde_json::from_slice(&game_bytes).unwrap();
    let game_id = game_json["id"].as_i64().unwrap() as i32;

    // 3) finish game
    let req_finish = Request::builder()
        .method("POST")
        .uri(&format!("/api/games/{}/finish", game_id))
        .body(Body::empty())
        .unwrap();

    let res_finish = app.clone().oneshot(req_finish).await.unwrap();
    assert_eq!(res_finish.status(), StatusCode::OK);

    let finish_bytes = to_bytes(res_finish.into_body(), usize::MAX).await.unwrap();
    let finish_json: serde_json::Value = serde_json::from_slice(&finish_bytes).unwrap();

    assert_eq!(finish_json["id"].as_i64().unwrap() as i32, game_id);
    assert_eq!(finish_json["status"].as_str().unwrap(), "finished");
}