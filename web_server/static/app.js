const input = document.getElementById("wordInput");
const button = document.getElementById("sendBtn");
const responseBox = document.getElementById("serverResponse");

button.addEventListener("click", sendWord);
input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
        sendWord();
    }
});

async function sendWord() {
    const word = input.value.trim();

    if (word === "") {
        return;
    }

    try {
        const res = await fetch("/api/guess", {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify({ word })
        });

        const data = await res.json();

        responseBox.textContent = JSON.stringify(data, null, 2);
    } catch (err) {
        responseBox.textContent = "Erreur de communication avec le serveur";
    }

    input.value = "";
}