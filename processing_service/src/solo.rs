//! Session solo côté serveur (HashMap en RAM) : 6 tentatives valides max par mot,
//! hint sur la 1ʳᵉ lettre tant que la grille du mot est vide, série « win streak » reset si tu te plantes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub const SOLO_MAX_ATTEMPTS: u32 = 6;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoloAttemptDto {
    pub word: String,
    pub feedback: Vec<String>,
}

#[derive(Clone)]
pub struct SoloSession {
    pub user_id: i32,
    pub target: String,
    pub generation: u64,
    /// Essais valides (dictionnaire) pour le mot courant.
    pub attempts_this_word: u32,
    pub history: Vec<SoloAttemptDto>,
    /// Mots trouvés d’affilée sans échec sur un mot (remis à 0 si 6 essais épuisés sans succès).
    pub current_streak: i32,
}

impl SoloSession {
    pub fn first_letter_hint(&self) -> Option<String> {
        self.target.chars().next().map(|c| c.to_string())
    }
}

pub type SharedSolo = Arc<Mutex<HashMap<String, SoloSession>>>;

pub fn shared_solo() -> SharedSolo {
    Arc::new(Mutex::new(HashMap::new()))
}
