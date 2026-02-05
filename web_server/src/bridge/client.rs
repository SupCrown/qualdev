pub struct BackendResponse {
    pub message: String,
}

pub fn send(word: &str) -> BackendResponse {
    BackendResponse {
        message: format!("Serveur a reçu : {}", word),
    }
}
