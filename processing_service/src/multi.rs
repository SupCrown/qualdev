//! Room multi en mémoire : tout le monde joue le même mot, chacun sa grille (pas de fuite d’infos),
//! illimité en essais jusqu’au bon mot — premier à **5** mots trouvés clôt la partie.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

pub const MULTI_WORDS_TO_WIN: u8 = 5;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiWordsQuery {
    pub party_id: i32,
    pub user_id: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivateAttemptDto {
    pub word: String,
    pub feedback: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OtherScoreDto {
    pub user_id: i32,
    pub username: String,
    pub words_found: u8,
}

#[derive(Clone)]
pub struct PartyInner {
    pub party_id: i32,
    pub host_user_id: i32,
    pub target: String,
    pub generation: u64,
    pub words_found: HashMap<i32, u8>,
    pub private_attempts: HashMap<i32, Vec<PrivateAttemptDto>>,
    pub participants: HashSet<i32>,
    pub winner: Option<i32>,
}

impl PartyInner {
    pub fn first_letter_hint(&self) -> Option<String> {
        self.target.chars().next().map(|c| c.to_string())
    }
}

pub type SharedParties = Arc<Mutex<HashMap<i32, PartyInner>>>;

pub fn shared_parties() -> SharedParties {
    Arc::new(Mutex::new(HashMap::new()))
}
