const dictionary = [];
let answer = null;

const date = todayString();
let guesses = [""];

const ANSWER_COUNT = 2315;
const WORD_LENGTH = 5;
const GUESS_LIMIT = 6;

const FLIP_ANIMATION_DURATION = 500;
const DANCE_ANIMATION_DURATION = 500;

const gameMode = document.getElementById("game-mode");
const keyboard = document.querySelector("[data-keyboard]");
const alertContainer = document.querySelector("[data-alert-container]");
const guessGrid = document.querySelector("[data-guess-grid]");
const Response = { "Green": "green", "Yellow": "yellow", "Black": "black" };

// TODO:
//  - Consolidate animation methods
//  - Track statistics and show after game
//  - Link to analyze app after game complete
//  - Wrap as offline-friendly PWA 
//  - ? Separate data model (instead of inside DOM)
  // Guess array only? Guesses and responses? and keyboard?

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
    // Random: Choose a random word in the answer prefix of the word list
    answer = dictionary[Math.floor(Math.random() * ANSWER_COUNT)];
  } else {
    showAlert("Error: Unknown Game Mode");
  }

  if (mode === "Random") {
    guesses = [""];
  } else {
    guesses = JSON.parse(localStorage.getItem(`${date}-${mode}-guesses`)) || [""];
  }

  syncInterface();
  gameMode.blur();
  startInteraction()
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

  guesses[guesses.length - 1] += key;
}

function deleteKey() {
  const activeTiles = getActiveTiles();
  const lastTile = activeTiles[activeTiles.length - 1];
  if (lastTile == null) return;
  lastTile.textContent = "";
  delete lastTile.dataset.state;
  delete lastTile.dataset.letter;

  guesses[guesses.length - 1].slice(0, -1);
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

  if (answer !== guess && !dictionary.includes(guess)) {
    showAlert("Not in word list");
    shakeTiles(activeTiles);
    return;
  }

  guesses.push("");
  localStorage.setItem(`${date}-${gameMode.value}-guesses`, JSON.stringify(guesses));

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

  if (guesses.length > GUESS_LIMIT) {
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
