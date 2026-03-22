const LS_PLAYER = "tusmo_player";
const LS_SESSION = "tusmo_session_id";

const board = document.getElementById("board");
const input = document.getElementById("wordInput");
const sendBtn = document.getElementById("sendBtn");
const newGameBtn = document.getElementById("newGameBtn");
const errorLine = document.getElementById("errorLine");
const statusBanner = document.getElementById("statusBanner");
const toastEl = document.getElementById("toast");

let toastTimer = null;

function showToast(message) {
    if (!toastEl) return;
    toastEl.textContent = message;
    toastEl.hidden = false;
    requestAnimationFrame(() => {
        toastEl.classList.add("toast-visible");
    });
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
        toastEl.classList.remove("toast-visible");
        toastTimer = setTimeout(() => {
            toastEl.hidden = true;
            toastEl.textContent = "";
        }, 300);
    }, 3200);
}

let sessionId = null;
let wordLength = 5;
let maxAttempts = 5;
let gameStatus = "loading";
/** Première lettre affichée dans la ligne d’indices (début de partie). */
let firstLetterHint = null;

function showError(msg) {
    errorLine.hidden = false;
    errorLine.textContent = msg;
}

function clearError() {
    errorLine.hidden = true;
    errorLine.textContent = "";
}

function setBanner(text, kind) {
    if (!text) {
        statusBanner.hidden = true;
        statusBanner.textContent = "";
        statusBanner.className = "status-banner";
        return;
    }
    statusBanner.hidden = false;
    statusBanner.textContent = text;
    statusBanner.className = "status-banner " + (kind || "");
}

function setRowColumns(row) {
    if (!wordLength || wordLength < 1) return;
    row.style.gridTemplateColumns = `repeat(${wordLength}, minmax(0, 1fr))`;
}

function tileClass(fb) {
    if (fb === "correct") return "tile correct";
    if (fb === "present") return "tile present";
    return "tile absent";
}

function renderBoard(history) {
    board.innerHTML = "";
    const hist = history || [];
    const showLengthHint =
        gameStatus === "active" && hist.length === 0 && wordLength > 0;

    if (showLengthHint) {
        const hintRow = document.createElement("div");
        hintRow.className = "row row-length-hint";
        hintRow.setAttribute("aria-label", "Nombre de lettres à trouver");
        for (let i = 0; i < wordLength; i++) {
            const cell = document.createElement("div");
            const showLetter = i === 0 && firstLetterHint;
            cell.className = showLetter
                ? "tile placeholder placeholder-letter"
                : "tile placeholder";
            cell.textContent = showLetter ? firstLetterHint : "—";
            hintRow.appendChild(cell);
        }
        setRowColumns(hintRow);
        board.appendChild(hintRow);
    }

    for (const attempt of hist) {
        const row = document.createElement("div");
        row.className = "row";
        setRowColumns(row);
        const letters = [...attempt.word];
        const fb = attempt.feedback || [];
        for (let i = 0; i < wordLength; i++) {
            const cell = document.createElement("div");
            cell.className = tileClass(fb[i]);
            cell.textContent = letters[i] || "";
            row.appendChild(cell);
        }
        board.appendChild(row);
    }
}

function applyUiFromState(data) {
    sessionId = data.session_id;
    wordLength = data.word_length || 5;
    maxAttempts = data.max_attempts || 5;
    gameStatus = data.status || "active";
    firstLetterHint =
        data.first_letter_hint && String(data.first_letter_hint).length > 0
            ? String(data.first_letter_hint).charAt(0)
            : null;
    input.maxLength = wordLength;
    input.placeholder =
        wordLength > 0 ? wordLength + " lettres" : "Votre mot";

    if (sessionId != null) {
        localStorage.setItem(LS_SESSION, String(sessionId));
    }

    renderBoard(data.history || []);

    const finished = gameStatus === "won" || gameStatus === "lost";
    input.disabled = finished;
    sendBtn.disabled = finished;
    newGameBtn.hidden = !finished;

    if (gameStatus === "won") {
        setBanner("Bravo — mot trouvé !", "win");
    } else if (gameStatus === "lost") {
        const secret = data.target_word;
        setBanner(
            secret
                ? "Perdu — le mot était « " + secret + " »."
                : "Perdu — 5 essais épuisés.",
            "lose"
        );
    } else {
        setBanner("", null);
    }
}

async function apiStart() {
    const username = localStorage.getItem(LS_PLAYER);
    const body = username ? { username } : {};
    const res = await fetch("/api/game/start", {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify(body),
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) {
        throw new Error(data.error || res.statusText || "start échoué");
    }
    if (data.player_username) {
        localStorage.setItem(LS_PLAYER, data.player_username);
    }
    applyUiFromState({
        session_id: data.session_id,
        word_length: data.word_length,
        max_attempts: data.max_attempts,
        status: data.status || "active",
        history: data.history || [],
        first_letter_hint: data.first_letter_hint,
    });
}

async function apiState(id) {
    const res = await fetch("/api/game/state/" + encodeURIComponent(id), {
        headers: { Accept: "application/json" },
    });
    const data = await res.json().catch(() => ({}));
    if (res.status === 404) {
        return null;
    }
    if (!res.ok) {
        throw new Error(data.error || res.statusText);
    }
    return data;
}

async function apiGuess(word) {
    const res = await fetch("/api/guess", {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ session_id: sessionId, word }),
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) {
        const err = new Error(data.error || data.message || res.statusText);
        err.code = data.code;
        err.status = res.status;
        throw err;
    }
    applyUiFromState(data);
}

async function init() {
    clearError();
    sendBtn.disabled = true;
    try {
        const saved = localStorage.getItem(LS_SESSION);
        if (saved) {
            const st = await apiState(saved);
            if (st) {
                applyUiFromState(st);
                sendBtn.disabled = st.status !== "active";
                return;
            }
            localStorage.removeItem(LS_SESSION);
        }
        await apiStart();
    } catch (e) {
        showError(String(e.message || e));
    } finally {
        if (gameStatus === "active") {
            sendBtn.disabled = false;
        }
    }
}

async function sendWord() {
    const raw = input.value.trim();
    if (!raw || sessionId == null) return;
    if (raw.length !== wordLength) {
        showError("Le mot doit faire " + wordLength + " lettres.");
        return;
    }
    clearError();
    sendBtn.disabled = true;
    let keepInput = false;
    try {
        await apiGuess(raw);
    } catch (e) {
        if (e.code === "not_in_dictionary") {
            showToast("Ce mot n'est pas dans le dictionnaire.");
            keepInput = true;
        } else {
            showError(String(e.message || e));
        }
    } finally {
        if (!keepInput) {
            input.value = "";
        }
        if (gameStatus === "active") {
            sendBtn.disabled = false;
            input.focus();
        }
    }
}

async function newGame() {
    clearError();
    localStorage.removeItem(LS_SESSION);
    sessionId = null;
    newGameBtn.hidden = true;
    board.innerHTML = "";
    setBanner("", null);
    sendBtn.disabled = true;
    try {
        await apiStart();
    } catch (e) {
        showError(String(e.message || e));
    } finally {
        if (gameStatus === "active") {
            sendBtn.disabled = false;
        }
    }
}

sendBtn.addEventListener("click", sendWord);
newGameBtn.addEventListener("click", newGame);
input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") sendWord();
});

init();
