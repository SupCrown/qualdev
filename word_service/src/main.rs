fn main() {
    println!("Hello, world!");
}

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TrouveMotResponse {
    name: String,
    categorie: String,
    difficulty: u8,
}

async fn fetch_random_word(length: usize) -> Result<String, Box<dyn std::error::Error>> {
    let url = "https://trouve-mot.fr/api/random";

    loop {
        let response: Vec<TrouveMotResponse> =
            reqwest::get(url).await?.json().await?;

        let word = response[0].name.to_uppercase();

        if word.chars().count() == length {
            return Ok(word);
        }
    }
}