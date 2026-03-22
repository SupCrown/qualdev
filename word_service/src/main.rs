use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;
use tonic::{transport::Server, Request, Response, Status};

pub mod word {
    tonic::include_proto!("word");
}

use word::word_service_server::{WordService, WordServiceServer};
use word::{
    CheckWordRequest, CheckWordResponse, GetRandomWordRequest, GetRandomWordResponse,
};

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

    async fn check_word(
        &self,
        request: Request<CheckWordRequest>,
    ) -> Result<Response<CheckWordResponse>, Status> {
        let w = normalize(&request.into_inner().word);
        if w.is_empty() {
            return Ok(Response::new(CheckWordResponse { is_valid: false }));
        }
        let ok = is_valid_word(&w)
            .await
            .map_err(|_| Status::unavailable("échec vérification dictionnaire"))?;
        Ok(Response::new(CheckWordResponse { is_valid: ok }))
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

    while count > 8 {
        response = reqwest::get(url).await?.json().await?;
        count = response[0].name.chars().count();
    }

    Ok(normalize(&response[0].name))
}

async fn is_valid_word(word: &str) -> Result<bool, reqwest::Error> {
    let url = format!(
        "https://scrabble123.fr/dictionnaire-du-scrabble/{}",
        word.to_lowercase()
    );

    let body = reqwest::get(&url).await?.text().await?;

    let body_lower = body.to_lowercase();

    // Phrase positive exacte (page valide). Éviter « acceptable dans le jeu scrabble » seul :
    // les pages invalides contiennent « n'est pas acceptable dans le jeu Scrabble », qui inclut cette sous-chaîne.
    let strict_ok = body_lower.contains("est acceptable dans le jeu scrabble");

    Ok(strict_ok)
}