
// iPhone viewport is 852 x 393
const BOARD_WIDTH = 760;
const BOARD_HEIGHT = 380;
const BOARD_MARGIN = 16;

const HOLE_RADIUS = 4;
const PEG_RADIUS = 5;
const HOLE_SPACING = 13;
const TEAM_SPACING = 13;

const BUTTON_SPACING = (HOLE_SPACING * 15) / 5;
const BUTTON_SIZE = BUTTON_SPACING - 3;

// Each side of the board (top, right, bottom, left) has a range of holes.
// For each side:
//  - min and max hole on that side
//  - (x, y) for the first hole on this side
//  - (dx, dy) direction toward next hole on this side
//  - (tx, ty) direction toward next team hole series on this side
const HOLE_RANGES = [
  { min:   0 * 6/5, max:  40 * 6/5, x: BOARD_WIDTH/2 - (HOLE_SPACING * 20 * 6/5), y: BOARD_MARGIN,                dx: 1,  dy: 0,  tx: 0,  ty: 1 },
  { min:  40 * 6/5, max:  60 * 6/5, x: BOARD_WIDTH - BOARD_MARGIN, y: BOARD_HEIGHT/2 - (HOLE_SPACING * 10 * 6/5), dx: 0,  dy: 1,  tx: -1, ty: 0 },
  { min:  60 * 6/5, max: 100 * 6/5, x: BOARD_WIDTH/2 + (HOLE_SPACING * 20 * 6/5), y: BOARD_HEIGHT - BOARD_MARGIN, dx: -1, dy: 0,  tx: 0,  ty: -1 },
  { min: 100 * 6/5, max: 120 * 6/5, x: BOARD_MARGIN, y: BOARD_HEIGHT/2 + (HOLE_SPACING * 10 * 6/5),               dx: 0,  dy: -1, tx: 1,  ty: 0 },
]

const POSSIBLE_SCORES = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 20, 21, 22, 23, 24, 28, 29];

let state = {
  teams: [
    { index: 0, color: "--red", score: 0, last: 0 },
    { index: 1, color: "--blue", score: 0, last: 0 },
    { index: 2, color: "--white", score: 0, last: 0 }
  ]
};

const board = document.getElementById('board');
board.setAttribute('viewBox', `0 0 ${BOARD_WIDTH} ${BOARD_HEIGHT}`);

renderBoard();
document.addEventListener('keydown', handleKeyDown);

// Translate score (1-120) to a 'hole' on the board.
// There is a sixth hole after every five to make the groups of five.
function scoreToHole(score) {
  return Math.floor((score - 1) * 6/5) + 1;
}

function holePosition(team, hole) {
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

function renderBoard() {
  const parts = [];

  parts.push(`<rect x="0" y="0" width="${BOARD_WIDTH}" height="${BOARD_HEIGHT}" rx="16" ry="16" fill="#C9A876" />`);
  
  // Midpoint lines; uncomment to debug drawing alignment
  //parts.push(`<line x1="${BOARD_WIDTH / 2}" y1="0" x2="${BOARD_WIDTH / 2}" y2="${BOARD_HEIGHT}" stroke="rgba(0,0,0,0.25)" stroke-width="1"/>`);
  //parts.push(`<line x1="0" y1="${BOARD_HEIGHT / 2}" x2="${BOARD_WIDTH}" y2="${BOARD_HEIGHT / 2}" stroke="rgba(0,0,0,0.25)" stroke-width="1"/>`);
  
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
    const pS = holePosition(0, (score * 6/5));
    const pE = holePosition(lastTeam, (score * 6/5));
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
  const centerP = holePosition(lastTeam + 1, 24);

  const redC = holePosition(lastTeam + 1, 6);
  addButtons(parts, centerP.x - BUTTON_SPACING * 5.5, centerP.y, 0, "var(--red-dark)", "var(--red)");

  const blueC = holePosition(lastTeam + 1, 42);
  addButtons(parts, centerP.x + BUTTON_SPACING * 5.5, centerP.y, 1, "var(--blue-dark)", "var(--blue)");

  if (state.teams.length > 2) {
    addButtons(parts, centerP.x, centerP.y, 2, "var(--white-dark)", "var(--white)");
  }

  board.innerHTML = parts.join('');

  for (let team = 0; team < state.teams.length; team++) {    
    for (let score of POSSIBLE_SCORES) {
      let rect = document.getElementById(`team-${team}-${score}`).addEventListener("click", () => addToScore(team, score));
    }
  }
}

function circleForScore(team, score, radius, additional) {
  if (score < 0 || score > 120) { return ""; }

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

      parts.push(`<g id="team-${team}-${score}" transform="translate(${x}, ${y})" width="${BUTTON_SIZE}" height="${BUTTON_SIZE}" class="button">`);
      parts.push(`<rect fill="${color}" width="${BUTTON_SIZE}" height="${BUTTON_SIZE}" rx="3" />`);
      parts.push(`<text x="${BUTTON_SIZE / 2}" y="${BUTTON_SIZE / 2}" font-size="${BUTTON_SIZE / 2}" text-anchor="middle" dominant-baseline="central" fill="var(--foreground)">+${score}</text>`);
      parts.push(`</g>`);

      index += 1;
      x += BUTTON_SPACING;
    }
    y += BUTTON_SPACING;
  }
}

function addToScore(teamIndex, points) {
  var team = state.teams[teamIndex];
  team.last = team.score;
  team.score += points;

  const current = document.querySelector(`.team-${teamIndex}.current`);
  const last = document.querySelector(`.team-${teamIndex}.last`);

  current.classList.remove("current");
  current.classList.add("last");

  last.classList.add("current");
  last.classList.remove("last");

  const p = holePosition(teamIndex, scoreToHole(team.score));
  last.setAttribute('cx', p.x);
  last.setAttribute('cy', p.y);

  document.getElementById(`team-score-${teamIndex}`).textContent = team.score.toString();
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

function resetGame() {
  state = {
    teams: [
      { index: 0, color: "--red", score: 0, last: 0 },
      { index: 1, color: "--blue", score: 0, last: 0 },
      { index: 2, color: "--white", score: 0, last: 0 }
    ]
  };

  redrawPegs();
}

function handleKeyDown(event) {
  if (event.key === "Backspace") {
    resetGame();
  }
}