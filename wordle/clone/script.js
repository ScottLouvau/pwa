const dictionary = [];
let answer = null;

const ANSWER_COUNT = 2315;
const WORD_LENGTH = 5;
const GUESS_LIMIT = 6;

const FLIP_ANIMATION_DURATION = 500;
const DANCE_ANIMATION_DURATION = 500;

const gameMode = document.getElementById("game-mode");
const keyboard = document.querySelector("[data-keyboard]");
const alertContainer = document.querySelector("[data-alert-container]");
const guessGrid = document.querySelector("[data-guess-grid]");

// TODO:
//  - Consolidate animation methods
//  - Remember today's guesses; show in-progress game on reload during same day.
//  - Track statistics and show after game
//  - Link to analyze app after game complete
//  - Wrap as offline-friendly PWA 
//  - ? Separate data model (instead of inside DOM)

startup();

async function startup() {
  // Retrieve all words (answers are first, then other valid words)
  const words = await fetch('./data/words.txt').then((res) => res.text()).then((res) => res.split('\n'));
  dictionary.push(...words);

  // Choose an answer and start the game
  await chooseAnswer();
}

async function chooseAnswer() {
  const mode = gameMode.value;

  if (mode === "Global") {
    // Global: Fetch current answer
    const today = new Date().toLocaleDateString("sv");
    const official = await fetch(`https://scottlouvau.github.io/fetch/data/wordle/${today}.json`).then((res) => res.json());
    answer = official.solution;
  } else if (mode === "V1") {
    // V1: Choose an answer (from the answer prefix of the word list, moving down one answer each day)
    answer = dictionary[daysSinceLaunch() % ANSWER_COUNT];
  } else if (mode === "Random") {
    answer = dictionary[Math.floor(Math.random() * dictionary.length)];
  } else {
    showAlert("Error: Unknown Game Mode");
  }

  // Clear board
  guessGrid.innerHTML = "";
  for (let i = 0; i < WORD_LENGTH * GUESS_LIMIT; i++) {
    const tile = document.createElement("div");
    tile.classList.add("tile");
    guessGrid.appendChild(tile);
  }

  // Clear keyboard colors
  keyboard.querySelectorAll("[data-key]").forEach(key => {
    key.classList.remove("correct", "wrong", "wrong-location");
  });


  startInteraction()
}

function daysSinceLaunch() {
  const msPerDay = 1000 * 60 * 60 * 24;
  return Math.floor((new Date() - new Date(2021, 5, 19)) / msPerDay);
}

function startInteraction() {
  document.addEventListener("click", handleMouseClick);
  document.addEventListener("keydown", handleKeyPress);
  gameMode.addEventListener("change", chooseAnswer);
}

function stopInteraction() {
  document.removeEventListener("click", handleMouseClick);
  document.removeEventListener("keydown", handleKeyPress);
  gameMode.removeEventListener("change", chooseAnswer);
}

function handleMouseClick(e) {
  if (e.target.matches("[data-key]")) {
    pressKey(e.target.dataset.key);
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
  const activeTiles = getActiveTiles();
  if (activeTiles.length >= WORD_LENGTH) return;
  const nextTile = guessGrid.querySelector(":not([data-letter])");
  nextTile.dataset.letter = key.toLowerCase();
  nextTile.textContent = key;
  nextTile.dataset.state = "active";
}

function deleteKey() {
  const activeTiles = getActiveTiles();
  const lastTile = activeTiles[activeTiles.length - 1];
  if (lastTile == null) return;
  lastTile.textContent = "";
  delete lastTile.dataset.state;
  delete lastTile.dataset.letter;
}

function submitGuess() {
  const activeTiles = [...getActiveTiles()];
  if (activeTiles.length !== WORD_LENGTH) {
    showAlert("Not enough letters");
    shakeTiles(activeTiles);
    return;
  }

  const guess = activeTiles.reduce((word, tile) => {
    return word + tile.dataset.letter;
  }, "");

  if (!dictionary.includes(guess)) {
    showAlert("Not in word list");
    shakeTiles(activeTiles);
    return;
  }

  stopInteraction();
  activeTiles.forEach((...params) => flipTile(...params, guess));
}

function flipTile(tile, index, array, guess) {
  const letter = tile.dataset.letter;
  const key = keyboard.querySelector(`[data-key="${letter}"i]`);
  setTimeout(() => {
    tile.classList.add("flip")
  }, (index * FLIP_ANIMATION_DURATION) / 2);

  tile.addEventListener(
    "transitionend",
    () => {
      tile.classList.remove("flip");
      if (answer[index] === letter) {
        tile.dataset.state = "correct";
        key.classList.add("correct");
      } else if (answer.includes(letter)) {
        tile.dataset.state = "wrong-location";
        key.classList.add("wrong-location");
      } else {
        tile.dataset.state = "wrong";
        key.classList.add("wrong");
      }

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

function shakeTiles(tiles) {
  tiles.forEach(tile => {
    tile.classList.add("shake");
    tile.addEventListener(
      "animationend",
      () => {
        tile.classList.remove("shake");
      },
      { once: true }
    );
  });
}

function checkWinLose(guess, tiles) {
  if (guess === answer) {
    showAlert("You Win", 5000);
    danceTiles(tiles);
    stopInteraction();
    return;
  }

  const remainingTiles = guessGrid.querySelectorAll(":not([data-letter])")
  if (remainingTiles.length === 0) {
    showAlert(answer.toUpperCase(), null);
    stopInteraction();
  }
}

function danceTiles(tiles) {
  tiles.forEach((tile, index) => {
    setTimeout(() => {
      tile.classList.add("dance")
      tile.addEventListener(
        "animationend",
        () => {
          tile.classList.remove("dance");
        },
        { once: true }
      );
    }, (index * DANCE_ANIMATION_DURATION) / 5);
  });
}
