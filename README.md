# Tusmo — Jeu de mots (architecture distribuée)

Projet type **Tusmo / Wordle** : le joueur devine un mot en un nombre limité d’essais, avec feedback couleur par lettre. Ce dépôt regroupe plusieurs services Rust, une API PostgreSQL optionnelle et une interface web statique.

---

## Fonctionnalités fournies à l’utilisateur

### Données d’entrée

| Source | Données |
|--------|---------|
| **Navigateur** | Pseudo joueur (optionnel, stocké en `localStorage`), identifiant de session, mots proposés (longueur = celle du mot mystère). |
| **Variables d’environnement** | `DATABASE_URL` (PostgreSQL), `WORD_SERVICE_GRPC`, `PROCESSING_BASE`, `PROCESSING_BIND`, etc. |

### Traitements réalisés

1. **`word_service`** (gRPC) : tirage d’un mot aléatoire (API externe + normalisation) ; vérification qu’un mot existe dans le dictionnaire (page Scrabble, phrase positive stricte).
2. **`processing_service`** (HTTP) : crée une session en base, impose une longueur de mot **entre 4 et 8** lettres, valide chaque tentative via `CheckWord`, calcule le feedback **rouge / jaune / gris** (règles Wordle), limite à **5 essais**, révèle la **1ʳᵉ lettre** au départ et le **mot entier** en cas de défaite.
3. **`web_server`** : sert l’UI statique et **proxide** les appels JSON vers `processing_service`.
4. **`databases` / `game_api`** (optionnel) : API REST SQLx sur le schéma `game_db` (utilisateurs, sessions « lettres » — le jeu principal utilise plutôt l’accès direct PostgreSQL depuis `processing_service`).

### Données enregistrées

Schéma PostgreSQL **`game_db`** (via `processing_service` + migrations manuelles éventuelles) :

- **`users`** : `username` unique.
- **`game_sessions`** : `user_id`, `letters` (mot cible secret), `score` (nombre d’essais), `status` (`active` / `won` / `lost`).
- **`attempts`** : `session_id`, `word`, `is_valid`, `feedback` (JSON : tableau `correct` / `present` / `absent`).

### Données présentées à l’utilisateur

- Grille des tentatives avec couleurs (vert = bonne place, jaune = mal placée mais présente, gris = absente).
- Ligne d’indices initiale : **tirets** pour la longueur + **première lettre** visible.
- **Toast** si le mot n’est pas dans le dictionnaire (sans consommer d’essai).
- Bannière victoire / défaite (avec mot solution si défaite).
- Bouton **Nouvelle partie** après fin de manche.

---

## Architecture générale

```text
┌─────────────┐     HTTP (static + /api/*)      ┌─────────────┐
│  Navigateur │ ◄──────────────────────────────► │ web_server  │
└─────────────┘                                 │  :3000      │
                                                └──────┬──────┘
                                                       │ HTTP proxy
                                                       ▼
                                                ┌─────────────┐
                                                │ processing_ │
                                                │  service    │
                                                │  :3001      │
                                                └──────┬──────┘
                         ┌────────────────────────────┼────────────────┐
                         │ gRPC                       │ SQL            │
                         ▼                            ▼                │
                  ┌─────────────┐            ┌──────────────┐         │
                  │ word_service│            │  PostgreSQL  │         │
                  │  :50051     │            │  game_db     │         │
                  └─────────────┘            └──────────────┘         │
                                                                       │
                  (optionnel) game_api REST ◄─────────────────────────┘
```

### Services et interconnexions

| Service | Rôle | Protocole |
|---------|------|-----------|
| **web_server** | UI + reverse proxy vers le traitement | HTTP |
| **processing_service** | Orchestration jeu, persistance | HTTP + gRPC client + PostgreSQL |
| **word_service** | Mots et dictionnaire | gRPC (tonic + prost) |
| **game_api** (`databases/`) | CRUD exemple sur la même BDD | HTTP (Axum + SQLx) |

---

## Technologies utilisées

| Couche | Technologie | Pourquoi |
|--------|-------------|----------|
| UI | HTML, CSS, JavaScript (sans framework) | Légèreté, déploiement statique derrière Axum `ServeDir`, pas de chaîne de build front. |
| Web | **Axum**, **reqwest**, **tower-http** | Écosystème Rust async cohérent avec le reste du projet ; proxy HTTP simple. |
| Traitement | **Axum**, **SQLx**, **tonic** | SQLx pour PostgreSQL typé/async ; tonic pour gRPC inter-services. |
| Mots | **tonic**, **prost**, **reqwest**, **unicode-normalization** | Proto partagé ; appels HTTP sortants vers APIs de mots / Scrabble. |
| BDD | **PostgreSQL**, **SQLx** | Schéma relationnel explicite ; requêtes préparées. |

### Critères de choix pour l’IPC

- **HTTP (navigateur ↔ web_server ↔ processing)** : naturelle pour le navigateur (fetch, CORS same-origin), débogage avec `curl`, cache possible sur assets statiques.
- **gRPC (processing ↔ word_service)** : contrat fort via **Protobuf**, streaming/async natif avec **tonic**, typage généré côtier Rust. Alternative envisageable : HTTP REST sur `word_service` — rejetée car nous voulions utiliser plusieurs technologies,  factoriser les messages (`GetRandomWord`, `CheckWord`) et le format appel de fonction parraissait plus logique.

### Difficultés rencontrées et évolutions

- **Syntaxe des routes Axum 0.8** : les paramètres de chemin utilisent `{id}` et non `:id` (sinon panic au démarrage).
- **Validation dictionnaire** : faire valider qu'un mot est bien dans le dictionnaire a pris du temps car le fonctionnement de l'api nétait pas très clair.
- **Édition Rust 2024** : `std::env::set_var` dans les `build.rs` peut exiger un bloc `unsafe` ; **protoc-bin-vendored** évite d’imposer `protoc` système pour générer le code gRPC.

---

## Qualité du code

### Organisation des tests

| Crate | Emplacement | Rôle |
|-------|-------------|------|
| **processing_service** | `src/game_logic.rs` → `#[cfg(test)]` | Tests unitaires de la logique de feedback (rapides, sans I/O). |
| **processing_service** | `tests/game_rules.rs` | Tests d’intégration **légers** : scénarios combinant `compute_feedback` / `is_win` sans HTTP ni BDD. |
| **word_service** | `src/main.rs` → `mod tests` | Tests de **`normalize`** (casse, accents). |
| **databases (game_api)** | `tests/api_tests.rs` | Tests d’intégration HTTP avec **vraie BDD** (`DATABASE_URL`) : santé, création utilisateur, fin de session. |

### Tests unitaires — cas limites identifiés

Exemples dans `processing_service/src/game_logic.rs` :

- **Longueur différente** : `compute_feedback` retourne `None`.
- **Mot vide** : `("", "")` → `Some` de longueur 0 (cas limite trivial).
- **Un seul caractère faux** en fin de mot (`TUSMX` / `TUSMO`).
- **Doubles lettres** (`ABA` vs `AAB`) : quotas verts / jaunes cohérents avec Wordle.
- **Aucune lettre commune** : tout `Absent`.

Extraits :

```rust
// Longueur invalide
assert!(compute_feedback("ab", "abc").is_none());

// Une lettre incorrecte en dernière position
let f = compute_feedback("TUSMX", "TUSMO").unwrap();
assert_eq!(f[4], LetterFeedback::Absent);
```

### Tests d’intégration — scénarios

**`processing_service/tests/game_rules.rs`**

- Scénario **victoire** : même mot (casse différente) → tout vert + `is_win`.
- Scénario **informations partielles** : premier mot sans intersection avec la cible, second mot avec verts, jaunes et absents explicites (`SABLE` / `SALON`).

**`databases/tests/api_tests.rs`** (extrait de principe)

- `GET /health` → 200.
- Création d’un utilisateur avec nom unique horodaté + liste des utilisateurs contenant ce nom.
- Création de session + `POST .../finish` → statut `finished`.

Ces tests nécessitent une base accessible (`dotenv` + `DATABASE_URL`).

### Documentation

- **README** (ce fichier) : vue d’ensemble, flux de données, choix techniques, tests.
- **Documentation Rust** : commentaires `//!` sur `processing_service/src/lib.rs`, `main.rs`, `db.rs` ; commentaires `///` sur les fonctions publiques de `game_logic`.
- **Proto** : `word_service/proto/word.proto` et copie sous `processing_service/proto/` pour le client généré.

---

## Lancement rapide (développement)
1. S'assurer d'avoir toutes les dependances (notament libssl-dev)
2. `word_service` : `cargo run` (port **50051**).
3. `processing_service` : définir `DATABASE_URL` (ou défaut du projet), puis `cargo run` (**3001**).
4. `web_server` : `cargo run` (**3000**), ouvrir `http://127.0.0.1:3000`.

---

## Commandes de test

```bash
cd processing_service && cargo test
cd word_service && cargo test
cd databases && cargo test   # nécessite DATABASE_URL
```


