/**
 * UI UsTom — fetch vers /api (proxy web_server → processing_service).
 * Solo : session en mémoire côté serveur ; multi : poll léger pour les scores des autres.
 */
const LS_USER_ID = "tusmo_user_id";
const LS_USERNAME = "tusmo_username";
const LS_MODE = "tusmo_mode";
const LS_SOLO_SESSION = "tusmo_solo_session";
const LS_PARTY_ID = "tusmo_party_id";

const loginOverlay = document.getElementById("loginOverlay");
const loginName = document.getElementById("loginName");
const loginBtn = document.getElementById("loginBtn");
const loginError = document.getElementById("loginError");
const modeOverlay = document.getElementById("modeOverlay");
const modeSoloBtn = document.getElementById("modeSoloBtn");
const modeHostBtn = document.getElementById("modeHostBtn");
const modeJoinBtn = document.getElementById("modeJoinBtn");
const partyJoinInput = document.getElementById("partyJoinInput");
const modeError = document.getElementById("modeError");
const gamePanel = document.getElementById("gamePanel");
const profileBar = document.getElementById("profileBar");
const cycleInfo = document.getElementById("cycleInfo");
const board = document.getElementById("board");
const input = document.getElementById("wordInput");
const sendBtn = document.getElementById("sendBtn");
const newSoloBtn = document.getElementById("newSoloBtn");
const errorLine = document.getElementById("errorLine");
const statusBanner = document.getElementById("statusBanner");
const toastEl = document.getElementById("toast");
const sideAside = document.getElementById("sideAside");
const sideTitle = document.getElementById("sideTitle");
const sideHint = document.getElementById("sideHint");
const sideList = document.getElementById("sideList");
const changePlayerWrap = document.getElementById("changePlayerWrap");
const changePlayerBtn = document.getElementById("changePlayerBtn");

let toastTimer = null;
let pollTimer = null;

let userId = null;
let username = null;
/** @type {'solo'|'multi'|null} */
let gameMode = null;
let soloSessionId = null;
let partyId = null;

let wordLength = 5;
let firstLetterHint = null;

function showToast(message, kind) {
    if (!toastEl) return;
    toastEl.textContent = message;
    toastEl.hidden = false;
    toastEl.classList.toggle("toast-success", kind === "success");
    requestAnimationFrame(() => {
        toastEl.classList.add("toast-visible");
    });
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
        toastEl.classList.remove("toast-visible");
        toastTimer = setTimeout(() => {
            toastEl.hidden = true;
            toastEl.textContent = "";
            toastEl.classList.remove("toast-success");
        }, 300);
    }, kind === "success" ? 4200 : 3200);
}

function showError(msg) {
    errorLine.hidden = false;
    errorLine.textContent = msg;
}

function clearError() {
    errorLine.hidden = true;
    errorLine.textContent = "";
}

function setLoginError(msg) {
    if (!loginError) return;
    if (!msg) {
        loginError.hidden = true;
        loginError.textContent = "";
        return;
    }
    loginError.hidden = false;
    loginError.textContent = msg;
}

function setModeError(msg) {
    if (!modeError) return;
    if (!msg) {
        modeError.hidden = true;
        modeError.textContent = "";
        return;
    }
    modeError.hidden = false;
    modeError.textContent = msg;
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

function setRowColumns(row, len) {
    if (!len || len < 1) return;
    row.style.gridTemplateColumns = `repeat(${len}, minmax(0, 1fr))`;
}

function tileClass(fb) {
    if (fb === "correct") return "tile correct";
    if (fb === "present") return "tile present";
    return "tile absent";
}

/**
 * @param {Array} attempts
 * @param {{ showWho: boolean }} opts
 */
function renderBoard(attempts, opts) {
    const showWho = opts && opts.showWho;
    board.innerHTML = "";
    const hist = attempts || [];
    const showLengthHint = hist.length === 0 && wordLength > 0;

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
        setRowColumns(hintRow, wordLength);
        board.appendChild(hintRow);
    }

    for (const attempt of hist) {
        if (showWho && attempt.username) {
            const who = document.createElement("div");
            who.className = "attempt-who";
            who.textContent = attempt.username;
            board.appendChild(who);
        }

        const row = document.createElement("div");
        row.className = "row";
        setRowColumns(row, wordLength);
        const letters = [...String(attempt.word || "")];
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

function myBestFromLeaderboard(rows) {
    if (!username || !rows) return null;
    const me = rows.find((r) => r.username === username);
    return me ? me.best_score : null;
}

function applySoloState(data) {
    if (!data) return;
    soloSessionId = data.session_id;
    localStorage.setItem(LS_SOLO_SESSION, soloSessionId);
    wordLength = data.word_length || 5;
    const hint =
        data.first_letter_hint && String(data.first_letter_hint).length > 0
            ? String(data.first_letter_hint).charAt(0)
            : null;
    firstLetterHint = hint;
    input.maxLength = wordLength;
    input.placeholder = wordLength > 0 ? wordLength + " lettres" : "Votre mot";

    const maxA = data.max_attempts ?? 6;
    const used = data.attempts_used ?? 0;
    const streak = data.current_streak ?? 0;
    const gen = data.generation ?? 1;
    if (cycleInfo) {
        cycleInfo.hidden = false;
        cycleInfo.textContent =
            "Solo — série : " +
            streak +
            " | essais " +
            used +
            "/" +
            maxA +
            " | mot #" +
            gen;
    }

    renderBoard(data.history || [], { showWho: false });

    sideTitle.textContent = "Classement solo (séries)";
    sideHint.textContent =
        "Ton record = plus longue série de mots d’affilée sans te planter (6 tentatives max par mot).";
    if (sideList) {
        sideList.innerHTML = "";
        const rows = data.leaderboard || [];
        if (!rows.length) {
            const li = document.createElement("li");
            li.className = "leaderboard-empty";
            li.textContent = "—";
            sideList.appendChild(li);
        } else {
            for (const row of rows) {
                const li = document.createElement("li");
                const name = document.createElement("span");
                name.className = "lb-name";
                name.textContent = row.username;
                const score = document.createElement("span");
                score.className = "lb-score";
                score.textContent = String(row.best_score);
                li.appendChild(name);
                li.appendChild(score);
                sideList.appendChild(li);
            }
        }
    }

    if (profileBar && username) {
        const best = myBestFromLeaderboard(data.leaderboard);
        profileBar.innerHTML =
            "Profil <strong>" +
            escapeHtml(username) +
            "</strong>" +
            (best != null ? " — record série solo : <strong>" + best + "</strong>" : "");
    }

    setBanner("", null);
    if (newSoloBtn) newSoloBtn.hidden = false;
}

function applyMultiState(data) {
    if (!data) return;
    if (newSoloBtn) newSoloBtn.hidden = true;
    partyId = data.party_id;
    localStorage.setItem(LS_PARTY_ID, String(partyId));
    wordLength = data.word_length || 5;
    const hint =
        data.first_letter_hint && String(data.first_letter_hint).length > 0
            ? String(data.first_letter_hint).charAt(0)
            : null;
    firstLetterHint = hint;
    input.maxLength = wordLength;
    input.placeholder = wordLength > 0 ? wordLength + " lettres" : "Votre mot";

    if (cycleInfo) {
        cycleInfo.hidden = false;
        const y = data.your_words_found ?? 0;
        const gen = data.generation ?? 1;
        cycleInfo.textContent =
            "Partie n°" +
            partyId +
            " — tes mots : " +
            y +
            "/5 — en cours (mot série " +
            gen +
            ")";
    }

    renderBoard(data.your_attempts || [], { showWho: false });

    sideTitle.textContent = "Les autres (multi)";
    sideHint.textContent =
        "On affiche seulement le compteur de mots trouvés — pas leurs grilles, c’est volontaire.";
    sideList.innerHTML = "";
    const others = data.others_scores || [];
    if (!others.length) {
        const li = document.createElement("li");
        li.className = "leaderboard-empty";
        li.textContent = "Pour l’instant tu es seul·e dans la room";
        sideList.appendChild(li);
    } else {
        for (const o of others) {
            const li = document.createElement("li");
            const name = document.createElement("span");
            name.className = "lb-name";
            name.textContent = o.username;
            const score = document.createElement("span");
            score.className = "lb-score";
            score.textContent = String(o.words_found);
            li.appendChild(name);
            li.appendChild(score);
            sideList.appendChild(li);
        }
    }

    if (profileBar && username) {
        profileBar.innerHTML =
            "Profil <strong>" +
            escapeHtml(username) +
            "</strong> — code à coller aux autres : <strong>" +
            partyId +
            "</strong>";
    }

    if (data.finished && data.winner_user_id != null) {
        const youWon = data.winner_user_id === userId;
        setBanner(
            youWon
                ? "GG — premier à 5 mots, la win est pour toi."
                : "Fin de partie — quelqu’un d’autre a verrouillé les 5 mots avant toi.",
            youWon ? "win" : "lose"
        );
        input.disabled = true;
        sendBtn.disabled = true;
    } else {
        setBanner("", null);
        input.disabled = false;
        sendBtn.disabled = false;
    }
}

function escapeHtml(s) {
    const d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
}

function stopPoll() {
    if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
    }
}

function startMultiPoll() {
    stopPoll();
    pollTimer = setInterval(() => {
        refreshMultiState().catch(() => {});
    }, 1500);
}

async function refreshMultiState() {
    if (partyId == null || userId == null) return;
    const res = await fetch(
        "/api/multi/state?party_id=" +
            encodeURIComponent(String(partyId)) +
            "&user_id=" +
            encodeURIComponent(String(userId)),
        { headers: { Accept: "application/json" } }
    );
    const data = await res.json().catch(() => ({}));
    if (!res.ok) {
        throw new Error(data.error || res.statusText);
    }
    applyMultiState(data);
}

function showLoginOnly() {
    stopPoll();
    loginOverlay.hidden = false;
    modeOverlay.hidden = true;
    gamePanel.hidden = true;
    profileBar.hidden = true;
    cycleInfo.hidden = true;
    sideAside.hidden = true;
    changePlayerWrap.hidden = true;
    sendBtn.disabled = true;
    input.disabled = true;
    userId = null;
    username = null;
    gameMode = null;
    soloSessionId = null;
    partyId = null;
}

function showModeChrome() {
    loginOverlay.hidden = true;
    modeOverlay.hidden = false;
    gamePanel.hidden = true;
    profileBar.hidden = false;
    cycleInfo.hidden = true;
    sideAside.hidden = true;
    changePlayerWrap.hidden = true;
    sendBtn.disabled = true;
    input.disabled = true;
    setModeError("");
}

function showGameChrome() {
    loginOverlay.hidden = true;
    modeOverlay.hidden = true;
    gamePanel.hidden = false;
    profileBar.hidden = false;
    cycleInfo.hidden = false;
    sideAside.hidden = false;
    changePlayerWrap.hidden = false;
    sendBtn.disabled = false;
    input.disabled = false;
}

async function apiProfile(name) {
    const res = await fetch("/api/profile", {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ username: name }),
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) {
        throw new Error(data.error || res.statusText || "Impossible de créer le profil (backend OK ?)");
    }
    return data;
}

async function doLogin() {
    setLoginError("");
    const raw = (loginName.value || "").trim();
    if (!raw) {
        setLoginError("Il faut un pseudo, sinon le serveur ne sait pas qui envoie les requêtes.");
        return;
    }
    loginBtn.disabled = true;
    try {
        const data = await apiProfile(raw);
        userId = data.user_id;
        username = data.username;
        if (userId == null) throw new Error("Réponse API bizarre : pas de user_id.");
        localStorage.setItem(LS_USER_ID, String(userId));
        localStorage.setItem(LS_USERNAME, username);
        profileBar.hidden = false;
        profileBar.innerHTML =
            "Profil <strong>" +
            escapeHtml(username) +
            "</strong> — record série solo : <strong>" +
            (data.best_score ?? 0) +
            "</strong>";
        showModeChrome();
    } catch (e) {
        setLoginError(String(e.message || e));
    } finally {
        loginBtn.disabled = false;
    }
}

async function startSolo() {
    setModeError("");
    modeSoloBtn.disabled = true;
    try {
        const res = await fetch("/api/solo/start", {
            method: "POST",
            headers: { "Content-Type": "application/json", Accept: "application/json" },
            body: JSON.stringify({ user_id: userId }),
        });
        const data = await res.json().catch(() => ({}));
        if (!res.ok) {
            throw new Error(data.error || res.statusText);
        }
        gameMode = "solo";
        localStorage.setItem(LS_MODE, "solo");
        showGameChrome();
        applySoloState(data);
        stopPoll();
    } catch (e) {
        setModeError(String(e.message || e));
    } finally {
        modeSoloBtn.disabled = false;
    }
}

async function hostMulti() {
    setModeError("");
    modeHostBtn.disabled = true;
    try {
        const res = await fetch("/api/multi/host", {
            method: "POST",
            headers: { "Content-Type": "application/json", Accept: "application/json" },
            body: JSON.stringify({ user_id: userId }),
        });
        const data = await res.json().catch(() => ({}));
        if (!res.ok) {
            throw new Error(data.error || res.statusText);
        }
        partyId = data.party_id;
        localStorage.setItem(LS_PARTY_ID, String(partyId));
        localStorage.setItem(LS_MODE, "multi");
        gameMode = "multi";
        showGameChrome();
        await refreshMultiState();
        startMultiPoll();
    } catch (e) {
        setModeError(String(e.message || e));
    } finally {
        modeHostBtn.disabled = false;
    }
}

async function joinMulti() {
    setModeError("");
    const raw = (partyJoinInput.value || "").trim();
    const pid = parseInt(raw, 10);
    if (Number.isNaN(pid) || pid < 1) {
        setModeError("Code invalide — vérifie l’id que ton pote t’a filé.");
        return;
    }
    modeJoinBtn.disabled = true;
    try {
        const res = await fetch("/api/multi/join", {
            method: "POST",
            headers: { "Content-Type": "application/json", Accept: "application/json" },
            body: JSON.stringify({ user_id: userId, party_id: pid }),
        });
        const data = await res.json().catch(() => ({}));
        if (!res.ok) {
            throw new Error(data.error || res.statusText);
        }
        partyId = data.party_id;
        localStorage.setItem(LS_PARTY_ID, String(partyId));
        localStorage.setItem(LS_MODE, "multi");
        gameMode = "multi";
        showGameChrome();
        await refreshMultiState();
        startMultiPoll();
    } catch (e) {
        setModeError(String(e.message || e));
    } finally {
        modeJoinBtn.disabled = false;
    }
}

async function sendWord() {
    const raw = input.value.trim();
    if (!raw || userId == null) return;
    if (raw.length !== wordLength) {
        showError("Longueur KO : il faut exactement " + wordLength + " lettres sur ce mot.");
        return;
    }
    clearError();
    sendBtn.disabled = true;
    let keepInput = false;
    try {
        if (gameMode === "solo") {
            if (!soloSessionId) return;
            const res = await fetch("/api/solo/guess", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                    Accept: "application/json",
                },
                body: JSON.stringify({
                    session_id: soloSessionId,
                    user_id: userId,
                    word: raw,
                }),
            });
            const data = await res.json().catch(() => ({}));
            if (!res.ok) {
                const err = new Error(data.error || data.message || res.statusText);
                err.code = data.code;
                throw err;
            }
            if (data.won_word) {
                showToast("Nice — mot trouvé, la série monte.", "success");
            } else if (data.lost_word) {
                showToast("6 tentatives cramées sur ce mot : série à zéro, on enchaîne sur un nouveau.");
            }
            if (data.state) {
                applySoloState(data.state);
            }
        } else if (gameMode === "multi") {
            if (partyId == null) return;
            const res = await fetch("/api/multi/guess", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                    Accept: "application/json",
                },
                body: JSON.stringify({
                    party_id: partyId,
                    user_id: userId,
                    word: raw,
                }),
            });
            const data = await res.json().catch(() => ({}));
            if (!res.ok) {
                const err = new Error(data.error || data.message || res.statusText);
                err.code = data.code;
                throw err;
            }
            if (data.you_found_word) {
                showToast("Trouvé — tu passes au suivant avec les autres.", "success");
            }
            if (data.state) {
                applyMultiState(data.state);
            }
        }
    } catch (e) {
        if (e.code === "not_in_dictionary") {
            showToast("Mot absent du dico — pas compté comme tentative valide.");
            keepInput = true;
        } else {
            showError(String(e.message || e));
        }
    } finally {
        if (!keepInput) {
            input.value = "";
        }
        if (gameMode === "multi" && statusBanner && !statusBanner.hidden) {
            sendBtn.disabled = true;
        } else {
            sendBtn.disabled = false;
        }
        input.focus();
    }
}

async function newSoloGame() {
    clearError();
    await startSolo();
}

async function tryRestoreSession() {
    const savedId = localStorage.getItem(LS_USER_ID);
    const savedName = localStorage.getItem(LS_USERNAME);
    const mode = localStorage.getItem(LS_MODE);
    if (!savedId || !savedName) return false;

    userId = parseInt(savedId, 10);
    username = savedName;
    if (Number.isNaN(userId)) {
        localStorage.clear();
        return false;
    }

    profileBar.hidden = false;
    profileBar.innerHTML =
        "Profil <strong>" +
        escapeHtml(username) +
        "</strong>";

    if (mode === "solo") {
        const sid = localStorage.getItem(LS_SOLO_SESSION);
        if (!sid) return false;
        try {
            const res = await fetch("/api/solo/state/" + encodeURIComponent(sid), {
                headers: { Accept: "application/json" },
            });
            if (!res.ok) {
                localStorage.removeItem(LS_SOLO_SESSION);
                localStorage.removeItem(LS_MODE);
                return false;
            }
            const data = await res.json();
            gameMode = "solo";
            soloSessionId = sid;
            showGameChrome();
            applySoloState(data);
            stopPoll();
            return true;
        } catch {
            localStorage.removeItem(LS_SOLO_SESSION);
            localStorage.removeItem(LS_MODE);
            return false;
        }
    }

    if (mode === "multi") {
        const pid = localStorage.getItem(LS_PARTY_ID);
        if (!pid) return false;
        partyId = parseInt(pid, 10);
        if (Number.isNaN(partyId)) return false;
        try {
            const res = await fetch(
                "/api/multi/state?party_id=" +
                    encodeURIComponent(String(partyId)) +
                    "&user_id=" +
                    encodeURIComponent(String(userId)),
                { headers: { Accept: "application/json" } }
            );
            if (!res.ok) {
                localStorage.removeItem(LS_PARTY_ID);
                localStorage.removeItem(LS_MODE);
                return false;
            }
            const data = await res.json();
            gameMode = "multi";
            showGameChrome();
            applyMultiState(data);
            startMultiPoll();
            return true;
        } catch {
            localStorage.removeItem(LS_PARTY_ID);
            localStorage.removeItem(LS_MODE);
            return false;
        }
    }

    return false;
}

async function init() {
    clearError();
    const restored = await tryRestoreSession();
    if (restored) {
        return;
    }
    const savedId = localStorage.getItem(LS_USER_ID);
    const savedName = localStorage.getItem(LS_USERNAME);
    if (savedId && savedName) {
        userId = parseInt(savedId, 10);
        username = savedName;
        if (!Number.isNaN(userId)) {
            showModeChrome();
            return;
        }
    }
    showLoginOnly();
    loginName.focus();
}

loginBtn.addEventListener("click", doLogin);
loginName.addEventListener("keydown", (e) => {
    if (e.key === "Enter") doLogin();
});

modeSoloBtn.addEventListener("click", () => startSolo());
modeHostBtn.addEventListener("click", () => hostMulti());
modeJoinBtn.addEventListener("click", () => joinMulti());

changePlayerBtn.addEventListener("click", () => {
    localStorage.removeItem(LS_USER_ID);
    localStorage.removeItem(LS_USERNAME);
    localStorage.removeItem(LS_MODE);
    localStorage.removeItem(LS_SOLO_SESSION);
    localStorage.removeItem(LS_PARTY_ID);
    setLoginError("");
    setModeError("");
    loginName.value = "";
    partyJoinInput.value = "";
    stopPoll();
    board.innerHTML = "";
    showLoginOnly();
    loginName.focus();
});

newSoloBtn.addEventListener("click", () => newSoloGame());

sendBtn.addEventListener("click", sendWord);
input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") sendWord();
});

init();
