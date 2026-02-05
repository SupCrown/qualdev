use axum::{
    Router,
    routing::{get, post},
    extract::State,
    Json,
};
use sqlx::PgPool;

use crate::models::{User, NewUser};

pub fn routes(pool: PgPool) -> Router {
    Router::new()
        .route("/users", post(create_user))
        .route("/users", get(list_users))
        .with_state(pool)
}

async fn create_user(
    State(pool): State<PgPool>,
    Json(payload): Json<NewUser>,
) -> Json<User> {
    let user = sqlx::query_as!(
        User,
        r#"
        INSERT INTO game_db.users (username)
        VALUES ($1)
        RETURNING id, username
        "#,
        payload.username
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    Json(user)
}

async fn list_users(
    State(pool): State<PgPool>,
) -> Json<Vec<User>> {
    let users = sqlx::query_as!(
        User,
        r#"SELECT id, username FROM game_db.users"#
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    Json(users)
}
