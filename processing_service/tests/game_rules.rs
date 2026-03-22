//! Tests d’intégration **légers** sur la logique de jeu (sans HTTP, BDD ni gRPC).
//! Ils composent plusieurs propriétés du module `game_logic` comme dans un scénario utilisateur.

use processing_service::game_logic::{compute_feedback, is_win, LetterFeedback};

#[test]
fn scenario_win_after_identical_guess() {
    let target = "CHIEN";
    let guess = "chien";
    assert!(is_win(guess, target));
    let fb = compute_feedback(guess, target).unwrap();
    assert!(fb.iter().all(|c| *c == LetterFeedback::Correct));
}

#[test]
fn scenario_partial_then_full_information() {
    let target = "SALON";
    let first = compute_feedback("TIGRE", target).unwrap();
    assert!(first.iter().all(|x| *x == LetterFeedback::Absent));

    // S A L O N vs S A B L E → S,A verts ; B absent ; L jaune (mal placé) ; E absent
    let second = compute_feedback("SABLE", target).unwrap();
    assert_eq!(second[0], LetterFeedback::Correct);
    assert_eq!(second[1], LetterFeedback::Correct);
    assert_eq!(second[2], LetterFeedback::Absent);
    assert_eq!(second[3], LetterFeedback::Present);
    assert_eq!(second[4], LetterFeedback::Absent);
}
