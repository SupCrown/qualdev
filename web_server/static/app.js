const LS_PLAYER = "tusmo_player";
const LS_SESSION = "tusmo_session_id";

const board = document.getElementById("board");
const input = document.getElementById("wordInput");
const sendBtn = document.getElementById("sendBtn");
const newGameBtn = document.getElementById("newGameBtn");
const errorLine = document.getElementById("errorLine");
const statusBanner = document.getElementById("statusBanner");

let sessionId = null;
let wordLength = 5;
let maxAttempts = 5;
let gameStatus = "loading";

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

function tileClass(fb) {
    if (fb === "correct") return "tile correct";
    if (fb === "present") return "tile present";
    return "tile absent";
}

function renderBoard(history) {
    board.innerHTML = "";
    for (const attempt of history) {
        const row = document.createElement("div");
        row.className = "row";
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
    input.maxLength = wordLength;

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
        setBanner("Perdu — 5 essais épuisés.", "lose");
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
        throw new Error(data.error || data.message || res.statusText);
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
    try {
        await apiGuess(raw);
    } catch (e) {
        showError(String(e.message || e));
    } finally {
        input.value = "";
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
