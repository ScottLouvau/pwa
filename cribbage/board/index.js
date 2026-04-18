(() => {
  'use strict';

  const BOARD_WIDTH = 840;
  const BOARD_HEIGHT = 380;
  const BOARD_MARGIN_X = 48;
  const BOARD_MARGIN_Y = 24;

  const HOLE_RADIUS = 4;
  const HOLE_SPACING = 14;
  const GROUP_SPACING = 16;
  const TEAM_SPACING = 24;

  const board = document.getElementById('board');
  board.setAttribute('viewBox', `0 0 ${BOARD_WIDTH} ${BOARD_HEIGHT}`);

  function holePosition(team, score) {
    const hole = score - 1;
    const group = Math.floor(hole / 5);
    const holeInGroup = hole % 5;

    let x, y;

    if (score <= 40) {
      // Top, left-to-right
      x = BOARD_MARGIN_X + GROUP_SPACING * group + HOLE_SPACING * hole;
      y = BOARD_MARGIN_Y;
    } else if (score <= 60) {
      // Right, top-to-bottom
      x = BOARD_WIDTH - BOARD_MARGIN_X;
      y = BOARD_MARGIN_Y + GROUP_SPACING * (group - 8)  + HOLE_SPACING * (hole - 40);
    } else if (score <= 100) {
      // Bottom, right-to-left
      x = BOARD_WIDTH - (BOARD_MARGIN_X + GROUP_SPACING * (group - 12) + HOLE_SPACING * (hole - 60));
      y = BOARD_HEIGHT - BOARD_MARGIN_Y;
    } else {
      // Right, top-to-bottom
      x = BOARD_MARGIN_X;
      y = BOARD_HEIGHT - (BOARD_MARGIN_Y + GROUP_SPACING * (group - 20)  + HOLE_SPACING * (hole - 100));   
    }
    
    return { x: x, y: y };
  }

  function renderBoard() {
    const parts = [];

    parts.push(`<rect x="0" y="0" width="${BOARD_WIDTH}" height="${BOARD_HEIGHT}" rx="16" ry="16" fill="#C9A876" />`);
    parts.push(`<g fill="#3A2A1A">`);

    // Top
    let x = BOARD_MARGIN_X;
    for (let score = 1; score <= 120; score++) {
      const p = holePosition(0, score);
      parts.push(`<circle cx="${p.x}" cy="${p.y}" r="${HOLE_RADIUS}" />`);
    }

    // Right


    parts.push(`</g>`);

    board.innerHTML = parts.join('');
  }

  renderBoard();
})();
