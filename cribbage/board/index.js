(() => {
  'use strict';

  // iPhone viewport is 852 x 393
  const BOARD_WIDTH = 760;
  const BOARD_HEIGHT = 380;
  const BOARD_MARGIN = 16;

  const HOLE_RADIUS = 4;
  const PEG_RADIUS = 5;
  const HOLE_SPACING = 13;
  const TEAM_SPACING = 13;

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

  const board = document.getElementById('board');
  board.setAttribute('viewBox', `0 0 ${BOARD_WIDTH} ${BOARD_HEIGHT}`);

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

  function renderBoard(state) {
    const parts = [];

    parts.push(`<rect x="0" y="0" width="${BOARD_WIDTH}" height="${BOARD_HEIGHT}" rx="16" ry="16" fill="#C9A876" />`);
    
    // Midpoint lines; uncomment to debug drawing alignment
    //parts.push(`<line x1="${BOARD_WIDTH / 2}" y1="0" x2="${BOARD_WIDTH / 2}" y2="${BOARD_HEIGHT}" stroke="rgba(0,0,0,0.25)" stroke-width="1"/>`);
    //parts.push(`<line x1="0" y1="${BOARD_HEIGHT / 2}" x2="${BOARD_WIDTH}" y2="${BOARD_HEIGHT / 2}" stroke="rgba(0,0,0,0.25)" stroke-width="1"/>`);

    // Holes
    parts.push(`<g fill="#3A2A1A">`);
    for (let team = 0; team < state.teamCount; team++) {
      for (let score = 1; score <= 120; score++) {
        parts.push(circleForScore(team, score, HOLE_RADIUS));
      }
    }
    parts.push(`</g>`);

    // Lines, except in corners
    parts.push(`<g stroke="rgba(0,0,0,0.25)" stroke-width="1">`);
    const radius = HOLE_RADIUS + 4;
    const lastTeam = state.teamCount - 1;
    for (let score of [10, 20, 30, 50, 70, 80, 90, 110]) {
      const pS = holePosition(0, (score * 6/5));
      const pE = holePosition(lastTeam, (score * 6/5));
      const color = (score === 90 ? `stroke="var(--red-dark)"` : ``);
      parts.push(`<line x1="${pS.x - pS.tx * radius}" y1="${pS.y - pS.ty * radius}" x2="${pE.x + pE.tx * radius}" y2="${pE.y + pE.ty * radius}" ${color} />`);
    }
    parts.push(`</g>`);

    parts.push(circleForScore(0, state.redScore, PEG_RADIUS, `fill="var(--red)"`));
    parts.push(circleForScore(0, state.redLast, PEG_RADIUS, `fill="var(--red-dark)"`));
    parts.push(circleForScore(1, state.blueScore, PEG_RADIUS, `fill="var(--blue)"`));
    parts.push(circleForScore(1, state.blueLast, PEG_RADIUS, `fill="var(--blue-dark)"`));

    board.innerHTML = parts.join('');
  }

  function circleForScore(team, score, radius, additional) {
    if (score < 1 || score > 120) { return ""; }

    const hole = scoreToHole(score)
    const p = holePosition(team, hole);
    return `<circle cx="${p.x}" cy="${p.y}" r="${radius}" ${additional} />`;
  }

  let state = {
    teamCount: 2,
    redScore: 58,
    redLast: 54,
    blueScore: 69,
    blueLast: 68
  };

  renderBoard(state);
})();
