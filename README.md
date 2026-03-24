# UsTom / Tusmo — jeu de mots distribué (Rust)

Projet type **Wordle / Tusmo** : feedback couleur par lettre, mots de **4 à 8** lettres, stack **Rust** (Axum, SQLx, tonic), **PostgreSQL** pour les profils / parties, **gRPC** vers un service « mots ». L’interface est du **HTML/CSS/JS** statique servi par `web_server`, qui **proxy** tout le JSON vers `processing_service`.

> **En pratique** : on lance d’abord le backend (`word_service` + `processing_service`), puis un ou plusieurs `web_server` sur les ports qu’on veut — chaque frontal pointe vers le même `PROCESSING_BASE` si on veut partager la même instance de logique.

---

## Sommaire

1. [Architecture](#architecture)
2. [Modes de jeu : solo et multijoueur](#modes-de-jeu--solo-et-multijoueur)
3. [Variables d’environnement](#variables-denvironnement)
4. [Scripts de lancement](#scripts-de-lancement)
5. [Lancement manuel (sans scripts)](#lancement-manuel-sans-scripts)
6. [API HTTP (`processing_service`)](#api-http-processing_service)
7. [Base de données](#base-de-données)
8. [Tests](#tests)
9. [Limitations connues](#limitations-connues)
10. [Technologies](#technologies)

---

## Architecture

```text
┌──────────────┐   HTTP (fichiers statiques + /api/*)   ┌──────────────┐
│  Navigateur  │ ◄──────────────────────────────────► │  web_server  │
│              │                                       │  (WEB_BIND)  │
└──────────────┘                                       └──────┬───────┘
                                                              │ HTTP
                                                              │ PROCESSING_BASE
                                                              ▼
                                                       ┌──────────────┐
                                                       │ processing_  │
                                                       │   service    │
                                                       │ (PROCESSING_ │
                                                       │   BIND)      │
                                                       └──────┬───────┘
                              ┌───────────────────────────────┼───────────────────────┐
                              │ gRPC                          │ SQL                     │
                              ▼                               ▼                       │
                       ┌──────────────┐              ┌──────────────┐                │
                       │ word_service │              │ PostgreSQL   │                │
                       │   (gRPC)     │              │  game_db     │                │
                       └──────────────┘              └──────────────┘                │
```

| Composant | Rôle |
|-----------|------|
| **word_service** | Tirage de mots, normalisation, **CheckWord** (dictionnaire). |
| **processing_service** | Profils, sessions **solo**, parties **multi** (état en mémoire + persistance minimale en base). |
| **web_server** | `ServeDir` sur `static/` + reverse proxy vers `PROCESSING_BASE`. |

Le dossier **`databases/`** (API SQLx d’exemple) est optionnel ; le jeu principal parle à PostgreSQL depuis `processing_service`.

---

## Modes de jeu : solo et multijoueur

### Profil

- À la connexion : **pseudo uniquement** (pas de mot de passe). Création ou reprise du compte en base (`users`).
- Le navigateur garde `user_id` / pseudo en `localStorage` pour les requêtes suivantes.

### Solo

- **6 essais** par mot (soumissions **valides** au dictionnaire).
- **Première lettre** affichée tant qu’il n’y a pas encore de tentative sur ce mot.
- **Série** : nombre de mots trouvés **d’affilée** sans échec. Si tu **épuises les 6 essais** sur un mot, la série repart à **0** et un nouveau mot est tiré.
- **`best_score` en base** = **meilleure série** jamais atteinte (mise à jour seulement si tu bats ton record).

### Multijoueur

- **Héberger** : crée une ligne dans `game_db.parties` ; l’**`id`** de la ligne est le **code** à communiquer aux autres.
- **Rejoindre** : saisir ce code (entier).
- **Même mot** pour tout le monde ; **tentatives visibles uniquement pour toi** ; essais **illimités** jusqu’à trouver le mot.
- Quelqu’un trouve le mot → **nouveau mot pour tout le monde** (les grilles privées sont vidées).
- **Premier à 5 mots** gagne la partie.
- Panneau latéral : **nombre de mots trouvés** par les autres (pas leurs tentatives).

> **Note** : l’état détaillé des parties **multi** vit en **mémoire** dans `processing_service`. Un **redémarrage** du binaire fait perdre les parties en cours (le code peut encore exister en base, mais la partie active n’est pas rechargée automatiquement).

---

## Variables d’environnement

| Variable | Où | Défaut / sens |
|----------|-----|----------------|
| **`DATABASE_URL`** | `processing_service` | URL PostgreSQL (schéma `game_db`). |
| **`WORD_SERVICE_GRPC`** | `processing_service` | URL gRPC du `word_service`, ex. `http://127.0.0.1:50051`. |
| **`PROCESSING_BIND`** | `processing_service` | Adresse d’écoute HTTP, ex. `127.0.0.1:3001`. |
| **`PROCESSING_BASE`** | `web_server` | URL du `processing_service` pour le proxy, ex. `http://127.0.0.1:3001`. |
| **`WEB_BIND`** | `web_server` | Adresse d’écoute du frontal, ex. `127.0.0.1:3000`. |

---

## Scripts de lancement

Tous les scripts sont à la **racine du dépôt** (pense à `chmod +x` si besoin).

### 1. `run-backend.sh` — mots + traitement

Lance dans l’ordre :

1. `word_service` (gRPC, logs dans `.run-logs/word_service.log`)
2. `processing_service` (HTTP, logs dans `.run-logs/processing_service.log`)

Variables optionnelles : `WORD_HOST`, `WORD_PORT`, `PROCESSING_HOST`, `PROCESSING_PORT`, ou directement `WORD_SERVICE_GRPC` / `PROCESSING_BIND`.

```bash
./run-backend.sh
```

**Ctrl+C** arrête les deux processus.

### 2. `run-web.sh` — interface sur le port voulu

Lance **un** `web_server`. Sans argument : **`127.0.0.1:3000`**. Sinon : port seul (`8080` → `127.0.0.1:8080`) ou bind complet (`0.0.0.0:9000`).

```bash
./run-web.sh           # défaut : 127.0.0.1:3000
./run-web.sh 8080
./run-web.sh 0.0.0.0:9000
```

`PROCESSING_BASE` reste par défaut `http://127.0.0.1:3001` (surcharge possible : `PROCESSING_BASE=... ./run-web.sh 3000`).

### 3. `run-multi-servers.sh` — tout-en-un pour tests

Lance `word_service`, `processing_service`, puis **deux** instances de `web_server` (ports configurables via `WEB1_BIND`, `WEB2_BIND`, etc.). Pratique pour tester **plusieurs frontaux** derrière le même backend.

---

## Lancement manuel (sans scripts)

1. Dépendances système si besoin (ex. `libssl-dev` pour le TLS SQLx).
2. Terminal 1 — `word_service` : `cd word_service && cargo run` (port gRPC **50051** par défaut côté proto/serveur).
3. Terminal 2 — `processing_service` : `cd processing_service && cargo run` (HTTP **3001** si `PROCESSING_BIND` non défini).
4. Terminal 3 — `web_server` : `cd web_server && cargo run` ou `WEB_BIND=127.0.0.1:3000 PROCESSING_BASE=http://127.0.0.1:3001 cargo run`.

Ouvrir l’URL affichée par `web_server` (ex. `http://127.0.0.1:3000`).

---

## API HTTP (`processing_service`)

Les mêmes chemins sont exposés par **`web_server`** sous `/api/...` (proxy transparent).

| Méthode | Chemin | Description |
|---------|--------|-------------|
| `POST` | `/api/profile` | `{ "username": "..." }` → `user_id`, `username`, `best_score`. |
| `POST` | `/api/solo/start` | `{ "user_id": n }` → état solo (dont `session_id`). |
| `GET` | `/api/solo/state/{session_id}` | État courant de la session solo. |
| `POST` | `/api/solo/guess` | `{ "session_id", "user_id", "word" }` → `won_word`, `lost_word`, `state`. |
| `POST` | `/api/multi/host` | `{ "user_id" }` → `{ "party_id" }` (code partie). |
| `POST` | `/api/multi/join` | `{ "user_id", "party_id" }`. |
| `GET` | `/api/multi/state` | Query `party_id`, `user_id` — état multi + scores des autres. |
| `POST` | `/api/multi/guess` | `{ "party_id", "user_id", "word" }` → `you_found_word`, `state`. |

Erreurs fréquentes côté client : **502** si `processing_service` n’est pas joignable (mauvais `PROCESSING_BASE` ou service arrêté) ; **`not_in_dictionary`** sur une tentative rejetée par le dictionnaire.

---

## Base de données

- Schéma **`game_db`** (utilisateur PostgreSQL avec les droits adaptés).
- **`users`** : pseudo unique, **`best_score`** = meilleure **série solo** (voir ci-dessus).
- **`parties`** : une ligne par partie **multi** créée par « héberger » ; la clé **`id`** sert de code.
- D’anciennes tables (`game_sessions`, `attempts`) peuvent exister pour d’autres usages ; le flux actuel **solo** n’en dépend pas pour la persistance de partie (sessions solo en mémoire côté serveur).

Migrations légères au démarrage de `processing_service` : colonne `best_score` si absente, table `parties` si absente.

---

## Tests

```bash
cd processing_service && cargo test
cd word_service && cargo test
cd databases && cargo test   # nécessite DATABASE_URL
```

Les tests unitaires du feedback Tusmo/Wordle sont dans `processing_service/src/game_logic.rs` et `tests/game_rules.rs`.

---

## Limitations connues

- **Parties multi** : état **volatile** si le process `processing_service` redémarre.
- **Solo** : session identifiée par `session_id` côté serveur ; perte si redémarrage du backend (le client peut retomber sur l’écran de choix de mode).
- **CORS** : même origine via `web_server` ; accès direct au navigateur vers `processing_service` non prévu dans ce dépôt.

---

## Technologies

| Couche | Choix | Commentaire rapide |
|--------|--------|---------------------|
| UI | HTML, CSS, JS sans framework | Simple à shipper derrière Axum. |
| Front HTTP | Axum, reqwest, tower-http | Proxy vers un seul `PROCESSING_BASE`. |
| Logique jeu | Axum, SQLx, tonic | SQL async + client gRPC pour les mots. |
| gRPC | tonic, prost, proto partagé | Contrat clair pour `GetRandomWord` / `CheckWord`. |
| BDD | PostgreSQL + SQLx | Requêtes préparées, typage des lignes. |

### Pistes / galères possibles en projet

- Routes **Axum 0.8** : paramètres de chemin en `{id}`, pas `:id`.
- Génération **gRPC** sans `protoc` système : `protoc-bin-vendored` dans le `build.rs` du `word_service` / client.
- **`hidden` + CSS** : si un overlay a `display: flex` en CSS, prévoir `.overlay[hidden] { display: none !important; }` pour que l’attribut HTML masque vraiment la modale.

---

## Licence / contexte

Projet pédagogique (architecture distribuée, plusieurs langages / protocoles). Adapter `DATABASE_URL` et les ports à ton environnement avant une démo ou un rendu.
