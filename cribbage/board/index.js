(() => {
  'use strict';

  // iPhone viewport is 852 x 393
  const BOARD_WIDTH = 760;
  const BOARD_HEIGHT = 380;
  const BOARD_MARGIN_X = 16;
  const BOARD_MARGIN_Y = 16;

  const HOLE_RADIUS = 4;
  const HOLE_SPACING = 13;
  const TEAM_SPACING = 13;

  const board = document.getElementById('board');
  board.setAttribute('viewBox', `0 0 ${BOARD_WIDTH} ${BOARD_HEIGHT}`);

  function holePosition(team, score) {
    // Score is 1-based, but holes are 0-based
    const hole = score - 1;

    // Spacing between groups of five is a sixth missing hole after each group.
    const holeWithGaps = Math.floor(hole * 6 / 5) + 1;

    let x, y;

    if (score <= 40) {
      // 1-40 along top, left-to-right. Middle is 20.
      const fromCenter = holeWithGaps - (20 * 6 / 5);
      x = (BOARD_WIDTH / 2) + HOLE_SPACING * fromCenter;
      y = BOARD_MARGIN_Y + TEAM_SPACING * team;
    } else if (score <= 60) {
      // 41-60 along right, top-to-bottom. Middle is 50.
      const fromCenter = holeWithGaps - (50 * 6 / 5);
      x = BOARD_WIDTH - BOARD_MARGIN_X - TEAM_SPACING * team;
      y = (BOARD_HEIGHT / 2) + HOLE_SPACING * fromCenter;
    } else if (score <= 100) {
      // 61-100 along bottom, right-to-left. Middle is 80.
      const fromCenter = holeWithGaps - (80 * 6 / 5);
      x = (BOARD_WIDTH / 2) - HOLE_SPACING * fromCenter;
      y = BOARD_HEIGHT - BOARD_MARGIN_Y - TEAM_SPACING * team;
    } else {
      // 101-120 along right, bottom-to-top. Middle is 110.
      const fromCenter = holeWithGaps - (110 * 6 / 5);
      x = BOARD_MARGIN_X  + TEAM_SPACING * team;
      y = (BOARD_HEIGHT / 2) - HOLE_SPACING * fromCenter;   
    }
    
    return { x: x, y: y };
  }

  function renderBoard() {
    const parts = [];

    parts.push(`<rect x="0" y="0" width="${BOARD_WIDTH}" height="${BOARD_HEIGHT}" rx="16" ry="16" fill="#C9A876" />`);
    parts.push(`<line x1="${BOARD_WIDTH / 2}" y1="0" x2="${BOARD_WIDTH / 2}" y2="${BOARD_HEIGHT}" stroke="rgba(0,0,0,0.25)" stroke-width="1"/>`);
    parts.push(`<line x1="0" y1="${BOARD_HEIGHT / 2}" x2="${BOARD_WIDTH}" y2="${BOARD_HEIGHT / 2}" stroke="rgba(0,0,0,0.25)" stroke-width="1"/>`);
    parts.push(`<g fill="#3A2A1A">`);

    // Holes
    for (let team = 0; team <= 2; team++) {
      for (let score = 1; score <= 120; score++) {
        const p = holePosition(team, score);
        parts.push(`<circle cx="${p.x}" cy="${p.y}" r="${HOLE_RADIUS}" />`);
      }
    }

    parts.push(`</g>`);

    board.innerHTML = parts.join('');
  }

  renderBoard();
})();
