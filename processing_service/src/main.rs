use processing_service::api_client::{ApiClient, ApiError};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() -> Result<(), ApiError> {
    let api = ApiClient::new("http://localhost:3000/api");

    let unique_username = format!(
        "alice_{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let user = api.create_user(&unique_username).await?;
    println!("User: {} ({})", user.id, user.username);

    let session = api.create_game_session(user.id, "salon").await?;
    println!(
        "Session: {} letters={} score={}",
        session.id, session.letters, session.score
    );

    let attempt = api.submit_attempt(session.id, "SALON").await?;
    println!("Attempt: {:?}", attempt);

    let session2 = api.get_game_session(session.id).await?;
    println!(
        "Session after attempt: score={} status={}",
        session2.score, session2.status
    );

    let finished = api.finish_game_session(session.id).await?;
    println!("Finished session: id={} status={}", finished.id, finished.status);

    Ok(())
}