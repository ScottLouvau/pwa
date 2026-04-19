
// iPhone viewport is 852 x 393
const BOARD_WIDTH = 760;
const BOARD_HEIGHT = 380;
const BOARD_MARGIN = 16;

const HOLE_RADIUS = 4;
const PEG_RADIUS = 5;
const HOLE_SPACING = 13;
const TEAM_SPACING = 13;

const BUTTON_SIZE = 24;
const BUTTON_SPACING = 30;

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

let state =  {
  teams: [
    { index: 0, color: "--red", score: 0, last: 0 },
    { index: 1, color: "--blue", score: 0, last: 0 }
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
  const innerBoardMargin = BOARD_MARGIN + TEAM_SPACING * state.teams.length;
  let x = innerBoardMargin;

  for (let score = 1; score <= 5; score++) {
    parts.push(`<g id="team-0-${score}" transform="translate(${x}, ${innerBoardMargin})" width="${BUTTON_SIZE}" height="${BUTTON_SIZE}" class="button">`)
    parts.push(`<rect fill="var(--red-dark)" width="${BUTTON_SIZE}" height="${BUTTON_SIZE}" rx="3" />`);
    parts.push(`<text x="${BUTTON_SIZE / 2}" y="${BUTTON_SIZE / 2}" font-size="${BUTTON_SIZE / 2}" text-anchor="middle" dominant-baseline="central" fill="var(--foreground)">+${score}</text>`);
    parts.push(`</g>`);

    x += BUTTON_SPACING;
  }

  board.innerHTML = parts.join('');

  for (let team = 0; team < 1; team++) {
    for (let score = 1; score <= 5; score++) {
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
  }
}

function resetGame() {
  state = {
    teams: [
      { index: 0, color: "--red", score: 0, last: 0 },
      { index: 1, color: "--blue", score: 0, last: 0 }
    ]
  };

  redrawPegs();
}

function handleKeyDown(event) {
  if (event.key === "Backspace") {
    resetGame();
  }
}