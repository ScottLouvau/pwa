
// iPhone viewport is 852 x 393
const BOARD_WIDTH = 780;
const BOARD_HEIGHT = 380;
const BOARD_MARGIN = 16;

const HOLE_RADIUS = 4;
const PEG_RADIUS = 5;
const HOLE_SPACING = 13;
const TEAM_SPACING = 13;
const SCORE_TO_HOLE = 6 / 5;

const BUTTON_SPACING = (HOLE_SPACING * 15) / 5; // 39px (5 buttons across in 15 hole-spaces)
const BUTTON_SIZE = BUTTON_SPACING - 3;         // 36px
const ACTION_BUTTON_SIZE = BUTTON_SIZE * 3;     // 108px

// Each side of the board (top, right, bottom, left) has a range of holes.
// For each side:
//  - min and max hole on that side
//  - (x, y) for the first hole on this side
//  - (dx, dy) direction toward next hole on this side
//  - (tx, ty) direction toward next team hole series on this side
const HOLE_RANGES = [
  { min:   0 * SCORE_TO_HOLE, max:  40 * SCORE_TO_HOLE, x: BOARD_WIDTH/2 - (HOLE_SPACING * 20 * SCORE_TO_HOLE), y: BOARD_MARGIN,                dx: 1,  dy: 0,  tx: 0,  ty: 1 },
  { min:  40 * SCORE_TO_HOLE, max:  60 * SCORE_TO_HOLE, x: BOARD_WIDTH - BOARD_MARGIN, y: BOARD_HEIGHT/2 - (HOLE_SPACING * 10 * SCORE_TO_HOLE), dx: 0,  dy: 1,  tx: -1, ty: 0 },
  { min:  60 * SCORE_TO_HOLE, max: 100 * SCORE_TO_HOLE, x: BOARD_WIDTH/2 + (HOLE_SPACING * 20 * SCORE_TO_HOLE), y: BOARD_HEIGHT - BOARD_MARGIN, dx: -1, dy: 0,  tx: 0,  ty: -1 },
  { min: 100 * SCORE_TO_HOLE, max: 120 * SCORE_TO_HOLE, x: BOARD_MARGIN, y: BOARD_HEIGHT/2 + (HOLE_SPACING * 10 * SCORE_TO_HOLE),               dx: 0,  dy: -1, tx: 1,  ty: 0 },
]

const POSSIBLE_SCORES = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 20, 21, 22, 23, 24, 28, 29];

let state = {
  teams: [
    { index: 0, color: "--red", score: 0, last: 0 },
    { index: 1, color: "--blue", score: 0, last: 0 }
  ],
  undo: [],
  redo: []
};

const board = document.getElementById('board');
board.setAttribute('viewBox', `0 0 ${BOARD_WIDTH} ${BOARD_HEIGHT}`);

renderBoard();
document.addEventListener('keydown', handleKeyDown);

// ---- Render SVG Board -----

// Translate score (1-120) to a 'hole' on the board.
// There is a sixth hole after every five to make the groups of five.
function scoreToHole(score) {
  return Math.floor((score - 1) * SCORE_TO_HOLE) + 1;
}

function holePosition(team, hole) {
  if (hole > (120 * SCORE_TO_HOLE)) { hole = 120 * SCORE_TO_HOLE; }
  if (hole < 0) { hole = 0; }

  // Find which side of the board this hole is on
  const range = HOLE_RANGES.find(r => hole <= r.max);
  const fromStart = hole - range.min;

  // Compute the position of this specific hole
  let x = range.x + range.dx * fromStart * HOLE_SPACING;
  let y = range.y + range.dy * fromStart * HOLE_SPACING;

  // Adjust coordinates for the desired team
  x += range.tx * team * TEAM_SPACING;
  y += range.ty * team * TEAM_SPACING;

  return { x: x, y: y, dx: range.dx, dy: range.dy, tx: range.tx, ty: range.ty };
}

function buildBoardSVG() {
  const parts = [];

  parts.push(`<rect x="0" y="0" width="${BOARD_WIDTH}" height="${BOARD_HEIGHT}" rx="16" ry="16" fill="#C9A876" />`);

  // Holes
  parts.push(`<g fill="#3A2A1A">`);
  for (let team = 0; team < state.teams.length; team++) {
    for (let score = 1; score <= 120; score++) {
      parts.push(circleForScore(team, score, HOLE_RADIUS));
    }
  }
  parts.push(`</g>`);

  // Lines, except in corners
  parts.push(`<g stroke="rgba(0,0,0,0.25)" stroke-width="1">`);
  const radius = HOLE_RADIUS + 4;
  const lastTeam = state.teams.length - 1;
  for (let score of [10, 20, 30, 50, 70, 80, 90, 110]) {
    const pS = holePosition(0, score * SCORE_TO_HOLE);
    const pE = holePosition(lastTeam, score * SCORE_TO_HOLE);
    const color = (score === 90 ? `stroke="var(--red-dark)"` : ``);
    parts.push(`<line x1="${pS.x - pS.tx * radius}" y1="${pS.y - pS.ty * radius}" x2="${pE.x + pE.tx * radius}" y2="${pE.y + pE.ty * radius}" ${color} />`);
  }
  parts.push(`</g>`);

  // Pegs
  for (let team of state.teams) {
    parts.push(circleForScore(team.index, team.score, PEG_RADIUS, `fill="var(${team.color})" class="peg team-${team.index} current"`));
    parts.push(circleForScore(team.index, team.last, PEG_RADIUS, `fill="var(${team.color})" class="peg team-${team.index} last"`));
  }

  // Scoring Buttons
  const centerP = holePosition(state.teams.length, 24);

  addButtons(parts, centerP.x - BUTTON_SPACING * 5.5, centerP.y, 0, "var(--red-dark)", "var(--red)");
  addButtons(parts, centerP.x + BUTTON_SPACING * 5.5, centerP.y, 1, "var(--blue-dark)", "var(--blue)");

  if (state.teams.length > 2) {
    addButtons(parts, centerP.x, centerP.y, 2, "var(--white-dark)", "var(--white)");
  }

  // Action Buttons (centered on each of the four 10-point segments along the bottom)
  const actionsY = holePosition(3, 80 * SCORE_TO_HOLE).y - BUTTON_SIZE;
  const ctrToLeft = -ACTION_BUTTON_SIZE / 2;

  addButton(parts, "undo",   holePosition(0, 95 * SCORE_TO_HOLE).x + ctrToLeft, actionsY, ACTION_BUTTON_SIZE, "Undo", "var(--button)");
  addButton(parts, "new-2p", holePosition(0, 85 * SCORE_TO_HOLE).x + ctrToLeft, actionsY, ACTION_BUTTON_SIZE, "New 2P", "var(--button)");
  addButton(parts, "new-3p", holePosition(0, 75 * SCORE_TO_HOLE).x + ctrToLeft, actionsY, ACTION_BUTTON_SIZE, "New 3P", "var(--button)");
  addButton(parts, "redo",   holePosition(0, 65 * SCORE_TO_HOLE).x + ctrToLeft, actionsY, ACTION_BUTTON_SIZE, "Redo", "var(--button)");

  return parts;
}

function renderBoard() {
  board.innerHTML = buildBoardSVG().join('');
  attachEventHandlers();
}

function attachEventHandlers() {
  // Hook up scoring button event handlers
  for (let team = 0; team < state.teams.length; team++) {    
    for (let score of POSSIBLE_SCORES) {
      document.getElementById(`team-${team}-${score}`).addEventListener("click", () => addToScore(team, score));
    }
  }

  document.getElementById(`undo`).addEventListener("click", () => undo());
  document.getElementById(`new-2p`).addEventListener("click", () => resetGame(2));
  document.getElementById(`new-3p`).addEventListener("click", () => resetGame(3));
  document.getElementById(`redo`).addEventListener("click", () => redo());
}

function circleForScore(team, score, radius, additional) {
  if (score < 0) { return ""; }

  const hole = scoreToHole(score)
  const p = holePosition(team, hole);
  return `<circle cx="${p.x}" cy="${p.y}" r="${radius}" ${additional} />`;
}

function addButtons(parts, center, top, team, color, scoreColor) {
  const left = center - BUTTON_SPACING * 2.5;

  // Alignment Debug Line
  //parts.push(`<line x1="${center}" y1="0" x2="${center}" y2="${BOARD_HEIGHT}" stroke="rgba(0,0,0,0.25)" stroke-width="1"/>`);

  parts.push(`<g transform="translate(${center - BUTTON_SIZE / 2}, ${top})" width="${BUTTON_SIZE}" height="${BUTTON_SIZE}" class="button">`);
  parts.push(`<text id="team-score-${team}" x="${BUTTON_SIZE / 2}" y="${BUTTON_SIZE / 2}" font-size="${BUTTON_SIZE}" text-anchor="middle" dominant-baseline="central" fill="${scoreColor}">0</text>`);
  parts.push(`</g>`);

  let index = 0;
  let y = top + BUTTON_SPACING;

  for (let row = 1; row <= 5; row++) {
    let x = left;

    for (let col = 1; col <= 5; col++) {
      let score = POSSIBLE_SCORES[index];
      addButton(parts, `team-${team}-${score}`, x, y, BUTTON_SIZE, `+${score}`, color);

      index += 1;
      x += BUTTON_SPACING;
    }
    y += BUTTON_SPACING;
  }
}

function addButton(parts, id, x, y, width, text, color) {
  parts.push(`<g id="${id}" transform="translate(${x}, ${y})" width="${width}" height="${BUTTON_SIZE}" class="button">`);
  parts.push(`<rect fill="${color}" width="${width}" height="${BUTTON_SIZE}" rx="3" />`);
  parts.push(`<text x="${width / 2}" y="${BUTTON_SIZE / 2}" font-size="${BUTTON_SIZE / 2}" text-anchor="middle" dominant-baseline="central" fill="var(--foreground)">${text}</text>`);
  parts.push(`</g>`);
}

function redrawPegs() {
  for (let team of state.teams) {
    const current = document.querySelector(`.team-${team.index}.current`);
    const cP = holePosition(team.index, scoreToHole(team.score));
    current.setAttribute('cx', cP.x);
    current.setAttribute('cy', cP.y);

    const last = document.querySelector(`.team-${team.index}.last`);
    const lP = holePosition(team.index, scoreToHole(team.last));
    last.setAttribute('cx', lP.x);
    last.setAttribute('cy', lP.y);

    document.getElementById(`team-score-${team.index}`).textContent = team.score.toString();
  }
}

function showScoreIncrease(teamIndex, toNewScore) {
  // Move back peg forward to new score (and swap which is 'back' peg)
  const last = document.querySelector(`.team-${teamIndex}.last`);
  swapCurrentAndLastPegs(teamIndex);
  
  const p = holePosition(teamIndex, scoreToHole(toNewScore));
  last.setAttribute('cx', p.x);
  last.setAttribute('cy', p.y);

  // Update shown score
  document.getElementById(`team-score-${teamIndex}`).textContent = toNewScore.toString();
}

function showScoreDecrease(teamIndex, toNewScore, toNewLastScore) {
  // Move current peg back to old last score (and swap which is 'back' peg)
  const current = document.querySelector(`.team-${teamIndex}.current`);
  swapCurrentAndLastPegs(teamIndex);
  
  const p = holePosition(teamIndex, scoreToHole(toNewLastScore));
  current.setAttribute('cx', p.x);
  current.setAttribute('cy', p.y);

  // Update shown score
  document.getElementById(`team-score-${teamIndex}`).textContent = toNewScore.toString();
}

// ---- EVENTS ----

function swapCurrentAndLastPegs(teamIndex) {
  const current = document.querySelector(`.team-${teamIndex}.current`);
  const last = document.querySelector(`.team-${teamIndex}.last`);

  current.classList.remove("current");
  current.classList.add("last");

  last.classList.add("current");
  last.classList.remove("last");
}

function addToScore(teamIndex, points) {
  var team = state.teams[teamIndex];
  
  state.undo.push({ team: team.index, last: team.last, score: team.score });
  state.redo = [];

  team.last = team.score;
  team.score += points;

  showScoreIncrease(teamIndex, team.score);
}

function undo() {
  if (state.undo.length === 0) { return; }
  let toUndo = state.undo.pop();

  var team = state.teams[toUndo.team];
  state.redo.push({ team: team.index, last: team.last, score: team.score });

  team.last = toUndo.last;
  team.score = toUndo.score;

  showScoreDecrease(team.index, team.score, team.last);
}

function redo() {
  if (state.redo.length === 0) { return; }
  let toRedo = state.redo.pop();

  var team = state.teams[toRedo.team];
  state.undo.push({ team: team.index, last: team.last, score: team.score });

  team.last = toRedo.last;
  team.score = toRedo.score;

  showScoreIncrease(toRedo.team, team.score);
}

function resetGame(teams) {
  const teamData = [
    { index: 0, color: "--red", score: 0, last: 0 },
    { index: 1, color: "--blue", score: 0, last: 0 }
  ];

  if (teams > 2) {
    teamData.push({ index: 2, color: "--white", score: 0, last: 0 });
  }

  state = { teams: teamData, undo: [], redo: [] };
  renderBoard();
}

function handleKeyDown(event) {
  if (event.key === "Backspace") {
    resetGame(2);
  }
}