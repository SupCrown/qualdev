use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;
use tonic::{transport::Server, Request, Response, Status};

pub mod word {
    tonic::include_proto!("word");
}

use word::word_service_server::{WordService, WordServiceServer};
use word::{GetRandomWordRequest, GetRandomWordResponse};

struct WordServiceImpl;

#[tonic::async_trait]
impl WordService for WordServiceImpl {
    async fn get_random_word(
        &self,
        _request: Request<GetRandomWordRequest>,
    ) -> Result<Response<GetRandomWordResponse>, Status> {
        let word = fetch_and_normalize_word()
            .await
            .map_err(|_| Status::unavailable("Failed to fetch word"))?;

        Ok(Response::new(GetRandomWordResponse { word }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse()?;

    let service = WordServiceImpl;

    println!("🚀 WordService listening on {}", addr);

    Server::builder()
        .add_service(WordServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}


#[derive(Debug, Deserialize)]
struct TrouveMotResponse {
    name: String,
}

fn normalize(word: &str) -> String {
    let filtered: String = word
        .nfd()
        .filter(|c| c.is_ascii_alphabetic())
        .collect();

    filtered.as_str().to_uppercase()
}

async fn fetch_and_normalize_word() -> Result<String, reqwest::Error> {
    let url = "https://trouve-mot.fr/api/sizemin/4";

    let mut response: Vec<TrouveMotResponse> =
        reqwest::get(url).await?.json().await?;
    let mut count = response[0].name.chars().count();

    while count > 10 {
        response = reqwest::get(url).await?.json().await?;
        count = response[0].name.chars().count();
    }

    Ok(normalize(&response[0].name))
}