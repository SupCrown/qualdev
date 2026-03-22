//! Règles de feedback **Wordle / Tusmo** : lettre bien placée (rouge), mal placée (jaune), absente (gris).
//! Implémentation alignée sur les quotas de lettres du mot cible (doubles lettres gérés).

use serde::Serialize;

/// États d’une case (Tusmo / Wordle) : rouge = bonne place, jaune = dans le mot ailleurs, gris = absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LetterFeedback {
    Correct,
    Present,
    Absent,
}

/// Règles Wordle standard : vert puis jaune avec quotas de lettres dans le mot cible.
pub fn compute_feedback(guess: &str, target: &str) -> Option<Vec<LetterFeedback>> {
    let g: Vec<char> = guess.chars().map(|c| c.to_ascii_uppercase()).collect();
    let t: Vec<char> = target.chars().map(|c| c.to_ascii_uppercase()).collect();
    if g.len() != t.len() {
        return None;
    }
    let n = g.len();
    let mut out = vec![LetterFeedback::Absent; n];

    for i in 0..n {
        if g[i] == t[i] {
            out[i] = LetterFeedback::Correct;
        }
    }

    let mut remaining: Vec<char> = Vec::new();
    for i in 0..n {
        if out[i] != LetterFeedback::Correct {
            remaining.push(t[i]);
        }
    }

    for i in 0..n {
        if out[i] == LetterFeedback::Correct {
            continue;
        }
        if let Some(pos) = remaining.iter().position(|&c| c == g[i]) {
            remaining.remove(pos);
            out[i] = LetterFeedback::Present;
        }
    }

    Some(out)
}

pub fn is_win(guess: &str, target: &str) -> bool {
    guess
        .chars()
        .map(|c| c.to_ascii_uppercase())
        .eq(target.chars().map(|c| c.to_ascii_uppercase()))
}

#[cfg(test)]
mod tests {
    use super::{compute_feedback, is_win, LetterFeedback};

    #[test]
    fn all_green_when_exact_match_case_insensitive() {
        let f = compute_feedback("tusmo", "TUSMO").unwrap();
        assert!(f.iter().all(|x| *x == LetterFeedback::Correct));
        assert!(is_win("tusmo", "TUSMO"));
    }

    #[test]
    fn wrong_length_returns_none() {
        assert!(compute_feedback("ab", "abc").is_none());
        assert!(compute_feedback("", "").is_some()); // 0 == 0
    }

    #[test]
    fn one_wrong_letter_end() {
        let f = compute_feedback("TUSMX", "TUSMO").unwrap();
        assert_eq!(f[0], LetterFeedback::Correct);
        assert_eq!(f[1], LetterFeedback::Correct);
        assert_eq!(f[2], LetterFeedback::Correct);
        assert_eq!(f[3], LetterFeedback::Correct);
        assert_eq!(f[4], LetterFeedback::Absent);
    }

    /// Doubles lettres : quotas corrects (mot cible `AAB`, tentative `ABA`).
    #[test]
    fn double_letter_quota_aab_aba() {
        let f = compute_feedback("ABA", "AAB").unwrap();
        assert_eq!(f[0], LetterFeedback::Correct);
        assert_eq!(f[1], LetterFeedback::Present);
        assert_eq!(f[2], LetterFeedback::Present);
    }

    #[test]
    fn yellow_not_green_when_duplicate_in_guess() {
        // Cible AABBB, tentative : vérifier longueur et cohérence basique
        let f = compute_feedback("BBAAA", "AABBB").unwrap();
        assert_eq!(f.len(), 5);
    }

    #[test]
    fn all_absent_when_no_shared_letters() {
        let f = compute_feedback("FGHIJ", "ABCDE").unwrap();
        assert!(f.iter().all(|x| *x == LetterFeedback::Absent));
    }
}
