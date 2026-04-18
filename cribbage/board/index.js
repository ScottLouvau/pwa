(() => {
  'use strict';

  const BOARD_WIDTH = 852;
  const BOARD_HEIGHT = 393;

  const board = document.getElementById('board');
  board.setAttribute('viewBox', `0 0 ${BOARD_WIDTH} ${BOARD_HEIGHT}`);

  function renderBoard() {
    const parts = [];

    parts.push(`<rect x="0" y="0" width="${BOARD_WIDTH}" height="${BOARD_HEIGHT}" rx="16" ry="16" fill="#C9A876" />`);
    parts.push(`<g fill="#3A2A1A">`);

    // Top curve connecting outer(60) -> inner(61) for each team
    for (let t = 0; t < 5; t++) {
      parts.push(`<circle cx="${18 + 12 * t}" cy="18" r="5" />`);
    }

    parts.push(`</g>`);

    board.innerHTML = parts.join('');
  }

  renderBoard();
})();
