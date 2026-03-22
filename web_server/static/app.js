const input = document.getElementById("wordInput");
const button = document.getElementById("sendBtn");
const resultDisplay = document.getElementById("resultDisplay");
const errorLine = document.getElementById("errorLine");

button.addEventListener("click", sendWord);
input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
        sendWord();
    }
});

function setLoading(loading) {
    button.disabled = loading;
    input.disabled = loading;
    button.textContent = loading ? "…" : "Envoyer";
}

function showError(msg) {
    errorLine.hidden = false;
    errorLine.textContent = msg;
}

function clearError() {
    errorLine.hidden = true;
    errorLine.textContent = "";
}

async function sendWord() {
    const word = input.value.trim();

    if (word === "") {
        return;
    }

    clearError();
    setLoading(true);
    resultDisplay.textContent = "…";
    resultDisplay.classList.add("is-loading");

    try {
        const res = await fetch("/api/guess", {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
                Accept: "application/json",
            },
            body: JSON.stringify({ word }),
        });

        const raw = await res.text();
        let data;
        try {
            data = raw ? JSON.parse(raw) : {};
        } catch {
            showError(`Réponse invalide (${res.status}).`);
            resultDisplay.textContent = "—";
            return;
        }

        if (!res.ok) {
            const detail = data.message || raw || res.statusText;
            showError(`Erreur ${res.status} : ${detail}`);
            resultDisplay.textContent = "—";
            return;
        }

        if (typeof data.message === "string") {
            resultDisplay.textContent = data.message;
        } else {
            resultDisplay.textContent = JSON.stringify(data, null, 2);
        }
    } catch (err) {
        showError(
            "Impossible de joindre le serveur web. Vérifiez qu’il est lancé (port 3000) et que le service de traitement répond (port 3001 par défaut)."
        );
        resultDisplay.textContent = "—";
        console.error(err);
    } finally {
        resultDisplay.classList.remove("is-loading");
        setLoading(false);
        input.value = "";
        input.focus();
    }
}
