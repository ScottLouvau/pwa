const dictionary = [];
let answer = null;

const today = todayString();
let guesses = [""];

const ANSWER_COUNT = 2315;
const WORD_LENGTH = 5;
const GUESS_LIMIT = 6;

const FLIP_ANIMATION_DURATION = 500;

const gameMode = document.getElementById("game-mode");
const keyboard = document.querySelector("[data-keyboard]");
const alertContainer = document.querySelector("[data-alert-container]");
const guessGrid = document.querySelector("[data-guess-grid]");
const overlay = document.getElementById("overlay");
const Response = { "Green": "green", "Yellow": "yellow", "Black": "black" };

// iPad:
//  - Test caching behavior
//  - Verify works offline (V1 mode)

startup();

async function startup() {
  // Notify the browser of the associated service worker
  if ('serviceWorker' in navigator) {
    navigator.serviceWorker.register('./service-worker.js', { scope: './' });
  }

  // Retrieve all words (answers are first, then other valid words)
  const words = await fetch('./data/words.txt').then((res) => res.text()).then((res) => res.split('\n'));
  dictionary.push(...words);

  // Choose an answer and start the game
  await chooseAnswer();
}

async function chooseAnswer() {
  const mode = gameMode.value;
  gameMode.blur();

  if (mode === "Global") {
    // Global: Fetch current answer
    const official = await fetch(`https://scottlouvau.github.io/fetch/data/wordle/${today}.json`).then((res) => res.json());
    answer = official.solution;
  } else if (mode === "V1") {
    // V1: Choose an answer (from the answer prefix of the word list, moving down one answer each day)
    answer = dictionary[daysSinceLaunch() % ANSWER_COUNT];
  } else if (mode === "Random") {
    // Random: Choose a random word in the answer prefix of the word list
    answer = dictionary[Math.floor(Math.random() * ANSWER_COUNT)];
  } else {
    showAlert("Error: Unknown Game Mode");
  }

  if (mode === "Random") {
    guesses = [""];
  } else {
    let state = JSON.parse(localStorage.getItem(`${mode}-state`));
    guesses = (state?.date === today) ? state.guesses : [""];
  }

  syncInterface();
}

// Test: getResponse("papal", "apple") == [ "yellow", "yellow", "green", "black", "yellow" ]
//  P1 is yellow (matches unmatched P2 in apple)
//  P3 is green  (matches, right position)
//  A2 is yellow (uses up 'A' in answer)
//  A4 is black  (no more unmatched 'A')
//  L5 is black  (no 'L' in answer at all)
function getResponse(guess, answer) {
  if (guess.length < WORD_LENGTH) return null;

  let unmatched = {};
  for (let i = 0; i < guess.length; i++) {
    if (guess[i] !== answer[i]) {
      unmatched[answer[i]] = unmatched[answer[i]] + 1 || 1;
    }
  }

  let result = [];
  for (let i = 0; i < guess.length; i++) {
    if (guess[i] === answer[i]) {
      result.push(Response.Green);
    } else if (unmatched[guess[i]] > 0) {
      result.push(Response.Yellow);
      unmatched[guess[i]]--;
    } else {
      result.push(Response.Black);
    }
  }
  
  return result;
}

function syncInterface() {
  let responses = guesses.map((guess, index) => getResponse(guess, answer));
  let tiles = guessGrid.querySelectorAll(".tile");

  // Clear tiles
  for (tile of tiles) {
    tile.dataset.letter = "";
    tile.dataset.state = "";
    tile.classList.remove("green", "yellow", "black");
  }

  // Clear keyboard colors
  keyboard.querySelectorAll("[data-key]").forEach(key => {
    key.classList.remove("green", "yellow", "black");
  });
  
  // Re-add guesses
  for (let i = 0; i < GUESS_LIMIT; i++) {
    let guess = guesses[i] || "";
    let response = responses[i] || [];

    for (let j = 0; j < WORD_LENGTH; j++) {
      let letter = guess[j];
      let color = response[j];
      let tile = tiles[i * WORD_LENGTH + j];

      if (letter) {
        tile.textContent = letter;
        tile.dataset.letter = letter;
      } else {
        tile.textContent = "";
        delete tile.dataset.letter;
      }

      if (color) {
        tile.dataset.state = color;
        tile.classList.add(color);

        let key = keyboard.querySelector(`[data-key="${letter}"i]`);
        key.classList.add(color);
      } else {
        if (letter) {
          tile.dataset.state = "active";
        } else {
          delete tile.dataset.state;
        }
      }
    }
  }

  if (guesses.length <= GUESS_LIMIT && !guesses.includes(answer)) {
    startInteraction();
  }
}

function analyze() {
  let url = "https://scottlouvau.github.io/pwa/wordle-analyze/?g=" + guesses.join(",").replace(/,*$/, "");
  if (!url.endsWith(answer)) { url += "," + answer; }

  window.open(url, "_blank");
}

function todayString() {
  return new Date().toLocaleDateString("sv");
}

function daysSinceLaunch() {
  const msPerDay = 1000 * 60 * 60 * 24;
  return Math.floor((new Date() - new Date(2021, 5, 19)) / msPerDay);
}

function startInteraction() {
  document.addEventListener("click", handleMouseClick);
  document.addEventListener("keydown", handleKeyPress);
}

function stopInteraction() {
  document.removeEventListener("click", handleMouseClick);
  document.removeEventListener("keydown", handleKeyPress);
}

function handleMouseClick(e) {
  let target = e.target;
  while (target) {
    if (target.matches("[data-key]")) {
      pressKey(target.dataset.key);
      return;
    }

    if (e.target.matches("[data-enter]")) {
      submitGuess();
      return;
    }

    if (e.target.matches("[data-delete]")) {
      deleteKey();
      return;
    }

    target = target.parentElement;
  }
}

function handleKeyPress(e) {
  if (e.key === "Enter") {
    submitGuess();
    return;
  }

  if (e.key === "Backspace" || e.key === "Delete") {
    deleteKey();
    return;
  }

  if (e.key.match(/^[a-z]$/)) {
    pressKey(e.key);
    return;
  }
}

function pressKey(key) {
  key = key.toLowerCase();

  const activeTiles = getActiveTiles();
  if (activeTiles.length >= WORD_LENGTH) return;
  const nextTile = guessGrid.querySelector(":not([data-letter])");
  nextTile.dataset.letter = key;
  nextTile.textContent = key;
  nextTile.dataset.state = "active";

  guesses[guesses.length - 1] += key;
}

function deleteKey() {
  const activeTiles = getActiveTiles();
  const lastTile = activeTiles[activeTiles.length - 1];
  if (lastTile == null) return;
  lastTile.textContent = "";
  delete lastTile.dataset.state;
  delete lastTile.dataset.letter;

  guesses[guesses.length - 1] = guesses[guesses.length - 1].slice(0, -1) || "";
}

function submitGuess() {
  const activeTiles = [...getActiveTiles()];
  if (activeTiles.length !== WORD_LENGTH) {
    showAlert("Not enough letters");
    animate(activeTiles, "shake", 0);
    return;
  }

  const guess = guesses[guesses.length - 1];
  if (answer !== guess && !dictionary.includes(guess)) {
    showAlert("Not in word list");
    shakeTiles(activeTiles);
    return;
  }

  guesses.push("");

  if (gameMode.value !== "Random") {
    localStorage.setItem(`${gameMode.value}-state`, JSON.stringify({ date: today, guesses: guesses }));
  }

  stopInteraction();
  const response = getResponse(guess, answer);
  activeTiles.forEach((...params) => flipTile(...params, guess, response));
}

function flipTile(tile, index, array, guess, response) {
  const letter = tile.dataset.letter;
  const key = keyboard.querySelector(`[data-key="${letter}"i]`);
  setTimeout(() => {
    tile.classList.add("flip")
  }, (index * FLIP_ANIMATION_DURATION) / 2);

  tile.addEventListener(
    "transitionend",
    () => {
      tile.classList.remove("flip");
      tile.dataset.state = response[index];
      key.classList.add(response[index]);      

      if (index === array.length - 1) {
        tile.addEventListener(
          "transitionend",
          () => {
            startInteraction()
            checkWinLose(guess, array)
          },
          { once: true }
        );
      }
    },
    { once: true }
  );
}

function getActiveTiles() {
  return guessGrid.querySelectorAll('[data-state="active"]');
}

function showAlert(message, duration = 1000) {
  const alert = document.createElement("div");
  alert.textContent = message;
  alert.classList.add("alert");
  alertContainer.prepend(alert);
  if (duration == null) return;

  setTimeout(() => {
    alert.classList.add("hide");
    alert.addEventListener("transitionend", () => {
      alert.remove();
    })
  }, duration);
}

function checkWinLose(guess, tiles) {
  if (guess === answer) {
    showAlert("You Win", 5000);
    animate(tiles, "dance", 100);
    setTimeout(showStatistics, 2000);
  } else if (guesses.length > GUESS_LIMIT) {
    showAlert(answer.toUpperCase(), null);
  } else {
    // Game still in progress...
    return;
  }

  // Record the number of turns to solve (record[0] = 1 turn; record[6] is a loss)
  // ...guesses has an extra string for the next partial guess, so two strings => one guess => record[0].  
  let record = JSON.parse(localStorage.getItem("record")) || [0, 0, 0, 0, 0, 0, 0];
  record[guesses.length - 2]++;
  localStorage.setItem("record", JSON.stringify(record));

  stopInteraction();
}

function showStatistics() {
  const statistics = document.createElement("div");
  statistics.id = "statistics";
  statistics.classList.add("statistics");

  const label = document.createElement("div");
  label.textContent = "Statistics";
  label.classList.add("label");
  statistics.appendChild(label);

  const record = JSON.parse(localStorage.getItem("record")) || [0, 0, 0, 0, 0, 0, 0];
  const maxGameCount = Math.max(1, record.reduce((a, b) => Math.max(a, b)));
  
  for (let i = 0; i < record.length; i++) {
    const gameCount = record[i];
    const percentage = Math.floor(100 * gameCount / maxGameCount);

    const bar = document.createElement("div");
    bar.classList.add("bar");
    bar.style.width = `${percentage}%`;
    bar.textContent = `${gameCount}`;
    bar.dataset.count = gameCount;
    
    const turnCount = document.createElement("div");
    turnCount.textContent = `${i + 1}`;    

    const row = document.createElement("div");
    row.classList.add("row");
    row.appendChild(turnCount);
    row.appendChild(bar);
    statistics.appendChild(row);
  }

  const analyzeButton = document.createElement("button");
  analyzeButton.textContent = "Analyze";
  analyzeButton.addEventListener("click", analyze);
  statistics.appendChild(analyzeButton);
  
  overlay.appendChild(statistics);
  overlay.style.visibility = "visible";

  overlay.addEventListener("click", closeOverlay);
  statistics.addEventListener("click", (e) => e.stopPropagation());
}

function closeOverlay() {
  overlay.innerHTML = "";
  overlay.style.visibility = "hidden";
}

function animate(items, animationName, delayBetweenItemsMs) {
  items.forEach((item, index) => {
    setTimeout(() => {
      item.classList.add(animationName);
      item.addEventListener(
        "animationend",
        () => {
          item.classList.remove(animationName);
        },
        { once: true }
      );
    }, index * delayBetweenItemsMs);
  });
}

function deleteCaches() {
  navigator.serviceWorker.controller.postMessage("deleteCaches");
}