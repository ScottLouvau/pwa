(() => {
  'use strict';

  // ---------- Constants ----------
  const STORAGE_KEY = 'cribbageState';
  const WIN_SCORE = 120;           // first to 120 wins
  const SKUNK_THRESHOLD = 90;      // loser with < 90 is skunked
  const NUM_TEAMS = 2;

  // Board geometry (SVG user units)
  const HOLE_R = 2.2;
  const HOLE_SP = 6;       // vertical spacing between adjacent holes on a track
  const GROUP_GAP = 3;     // extra vertical gap between groups of 5
  const TRACK_SP = 10;     // horizontal distance between a team's outer and inner tracks
  const TEAM_GAP = 18;     // horizontal gap between the two teams
  const PAD_X = 8;
  const PAD_Y = 14;
  const PEG_R = 3.8;

  // Path layout: each team has a U-shape.
  //   Outer track: positions 0..60 (61 holes), from bottom to top
  //   Inner track: positions 61..120 (60 holes), from top to bottom
  // Curve at the top connects hole 60 -> hole 61.
  const OUTER_MAX = 60;

  // ---------- State ----------
  const state = {
    red_total: 0,
    blue_total: 0,
    history: [], // [{ team: 'red'|'blue', score: number }]
    winner: null, // 'red' | 'blue' | null
  };

  function save() {
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(state)); } catch (e) {}
  }

  function load() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return;
      const s = JSON.parse(raw);
      if (s && typeof s === 'object') {
        state.red_total = s.red_total || 0;
        state.blue_total = s.blue_total || 0;
        state.history = Array.isArray(s.history) ? s.history : [];
        state.winner = s.winner || null;
      }
    } catch (e) {}
  }

  function teamLastAdd(team) {
    for (let i = state.history.length - 1; i >= 0; i--) {
      if (state.history[i].team === team) return state.history[i].score;
    }
    return 0;
  }

  function teamTotal(team) {
    return team === 'red' ? state.red_total : state.blue_total;
  }

  function setTeamTotal(team, value) {
    if (team === 'red') state.red_total = value;
    else state.blue_total = value;
  }

  function addScore(team, amount) {
    if (state.winner) return; // game over
    setTeamTotal(team, teamTotal(team) + amount);
    state.history.push({ team, score: amount });
    const newTotal = teamTotal(team);
    if (newTotal >= WIN_SCORE) {
      state.winner = team;
    }
    save();
    render();
    bumpScore(team);
    if (state.winner) showGameOver();
  }

  function undo() {
    if (state.history.length === 0) return;
    const last = state.history.pop();
    setTeamTotal(last.team, teamTotal(last.team) - last.score);
    // If undoing took us below the win threshold, clear winner and hide modal
    if (state.winner && teamTotal(state.winner) < WIN_SCORE) {
      state.winner = null;
      closeOverlay(gameOverOverlay);
    }
    save();
    render();
    bumpScore(last.team);
  }

  function newGame() {
    state.red_total = 0;
    state.blue_total = 0;
    state.history = [];
    state.winner = null;
    save();
    render();
  }

  // ---------- Board geometry helpers ----------
  function outerYFromBot(i) { // i in 0..60, 0 at bottom
    return PAD_Y + i * HOLE_SP + Math.floor(i / 5) * GROUP_GAP;
  }
  function innerYFromBot(i) { // i in 61..120, 61 at top
    const j = i - 61; // 0..59 from top to bottom
    return outerYFromBot(OUTER_MAX) - j * HOLE_SP - Math.floor(j / 5) * GROUP_GAP;
  }

  const BOARD_H = outerYFromBot(OUTER_MAX) + PAD_Y;
  // Width: padding + (outer + track_sp + inner) * 2 teams + team_gap
  const BOARD_W = PAD_X * 2 + HOLE_R * 2 + TRACK_SP * 2 + TEAM_GAP;

  function outerX(teamIdx) {
    // teamIdx 0 = red (left), 1 = blue (right); outer is on the outer edge
    const redOuter = PAD_X + HOLE_R;
    const redInner = redOuter + TRACK_SP;
    const blueInner = redInner + TEAM_GAP;
    const blueOuter = blueInner + TRACK_SP;
    return teamIdx === 0 ? redOuter : blueOuter;
  }
  function innerX(teamIdx) {
    const redOuter = PAD_X + HOLE_R;
    const redInner = redOuter + TRACK_SP;
    const blueInner = redInner + TEAM_GAP;
    return teamIdx === 0 ? redInner : blueInner;
  }

  function holePos(teamIdx, i) {
    // i: 0..120
    const isInner = i > OUTER_MAX;
    const x = isInner ? innerX(teamIdx) : outerX(teamIdx);
    const yFromBot = isInner ? innerYFromBot(i) : outerYFromBot(i);
    return { x, y: BOARD_H - yFromBot };
  }

  // ---------- Rendering ----------
  const board = document.getElementById('board');
  board.setAttribute('viewBox', `0 0 ${BOARD_W} ${BOARD_H}`);

  function renderBoard() {
    const parts = [];

    parts.push(`<rect class="board-bg" x="0" y="0" width="${BOARD_W}" height="${BOARD_H}"/>`);

    // Top curve connecting outer(60) -> inner(61) for each team
    for (let t = 0; t < NUM_TEAMS; t++) {
      const p60 = holePos(t, 60);
      const p61 = holePos(t, 61);
      const midX = (p60.x + p61.x) / 2;
      const topY = Math.min(p60.y, p61.y) - (TRACK_SP / 2 + 2);
      parts.push(
        `<path class="track-arc" d="M ${p60.x} ${p60.y} C ${p60.x} ${topY}, ${p61.x} ${topY}, ${p61.x} ${p61.y}"/>`
      );
      // Tiny arrow-like indicator at the top curve (optional aesthetic)
      // Skipped for simplicity.
    }

    // Faint group lines (every 5 holes) drawn per track segment
    // Outer track: between holes (5k-1, 5k) for k = 1..12
    for (let t = 0; t < NUM_TEAMS; t++) {
      const ox = outerX(t);
      const ix = innerX(t);
      // Outer: lines at group boundaries on outer track
      for (let k = 1; k <= 12; k++) {
        const y1 = BOARD_H - outerYFromBot(5 * k - 1);
        const y2 = BOARD_H - outerYFromBot(5 * k);
        const y = (y1 + y2) / 2;
        parts.push(`<line class="group-line" x1="${ox - HOLE_R - 1}" y1="${y}" x2="${ox + HOLE_R + 1}" y2="${y}"/>`);
      }
      // Inner: group boundaries on inner track (between holes 5k+60 and 5k+61 -> i.e., 65-66, 70-71, ..., 115-116)
      for (let k = 1; k <= 11; k++) {
        const lowI = 60 + 5 * k;   // 65, 70, 75, ..., 115
        const highI = lowI + 1;    // 66, 71, 76, ..., 116
        const y1 = BOARD_H - innerYFromBot(lowI);
        const y2 = BOARD_H - innerYFromBot(highI);
        const y = (y1 + y2) / 2;
        parts.push(`<line class="group-line" x1="${ix - HOLE_R - 1}" y1="${y}" x2="${ix + HOLE_R + 1}" y2="${y}"/>`);
      }
    }

    // Skunk line: horizontal line across the whole board at y of hole 90 (inner).
    // Loser must reach 90 to avoid being skunked.
    const skunk90 = holePos(0, 90);
    const skunk89 = holePos(0, 89);
    const skunkY = (skunk90.y + skunk89.y) / 2;
    parts.push(`<line class="skunk-line" x1="2" y1="${skunkY}" x2="${BOARD_W - 2}" y2="${skunkY}"/>`);

    // Holes (0..120) on both tracks for both teams
    for (let t = 0; t < NUM_TEAMS; t++) {
      for (let i = 0; i <= 120; i++) {
        const p = holePos(t, i);
        parts.push(`<circle class="hole" cx="${p.x}" cy="${p.y}" r="${HOLE_R}"/>`);
      }
    }

    // Pegs (previous + current for each team)
    const teams = [
      { idx: 0, color: 'red',  total: state.red_total,  last: teamLastAdd('red')  },
      { idx: 1, color: 'blue', total: state.blue_total, last: teamLastAdd('blue') },
    ];
    for (const team of teams) {
      if (team.total <= 0) continue;
      const curI = Math.min(team.total, 120);

      // Previous peg (at the score BEFORE the most recent add for this team)
      if (team.last > 0) {
        const prevI = Math.max(0, Math.min(team.total - team.last, 120));
        const pp = holePos(team.idx, prevI);
        parts.push(`<circle class="peg-trail peg-${team.color}" cx="${pp.x}" cy="${pp.y}" r="${PEG_R}"/>`);
      }

      // Current peg
      const cp = holePos(team.idx, curI);
      parts.push(`<circle class="peg peg-${team.color}" cx="${cp.x}" cy="${cp.y}" r="${PEG_R}"/>`);
    }

    board.innerHTML = parts.join('');
  }

  function render() {
    document.getElementById('redScore').textContent = state.red_total;
    document.getElementById('blueScore').textContent = state.blue_total;
    renderBoard();
  }

  function bumpScore(team) {
    const el = document.getElementById(team === 'red' ? 'redScore' : 'blueScore');
    el.classList.remove('bump');
    void el.offsetWidth;
    el.classList.add('bump');
  }

  // ---------- Game over ----------
  const gameOverOverlay = document.getElementById('gameOverOverlay');
  const gameOverTitle = document.getElementById('gameOverTitle');
  const gameOverSubtitle = document.getElementById('gameOverSubtitle');
  const skunkMessage = document.getElementById('skunkMessage');

  function showGameOver() {
    const winner = state.winner;
    const loser = winner === 'red' ? 'blue' : 'red';
    const loserScore = teamTotal(loser);
    const winnerLabel = winner === 'red' ? 'Red' : 'Blue';
    const loserLabel = loser === 'red' ? 'Red' : 'Blue';
    gameOverTitle.textContent = `${winnerLabel} Wins!`;
    gameOverTitle.className = `winner-${winner}`;
    gameOverSubtitle.textContent = `${loserLabel} finished with ${loserScore}.`;
    if (loserScore < SKUNK_THRESHOLD) {
      skunkMessage.textContent = `${loserLabel} was skunked!`;
      skunkMessage.classList.remove('hidden');
    } else {
      skunkMessage.classList.add('hidden');
    }
    openOverlay(gameOverOverlay);
  }

  document.getElementById('closeGameOver').addEventListener('click', () => closeOverlay(gameOverOverlay));
  document.getElementById('newGameFromOver').addEventListener('click', () => {
    newGame();
    closeOverlay(gameOverOverlay);
  });

  // ---------- Buttons ----------
  function buildScoreButtons() {
    const red = document.getElementById('redButtons');
    const blue = document.getElementById('blueButtons');

    for (const col of [red, blue]) {
      const team = col === red ? 'red' : 'blue';
      for (let n = 1; n <= 9; n++) {
        const b = document.createElement('button');
        b.type = 'button';
        b.textContent = `+${n}`;
        b.addEventListener('click', () => addScore(team, n));
        col.appendChild(b);
      }
      const tenBtn = document.createElement('button');
      tenBtn.type = 'button';
      tenBtn.className = 'tenplus';
      tenBtn.textContent = '10+';
      tenBtn.addEventListener('click', () => openTenPlus(team));
      col.appendChild(tenBtn);
    }
  }

  // ---------- 10+ overlay ----------
  const tenPlusOverlay = document.getElementById('tenPlusOverlay');
  const tenPlusGrid = document.getElementById('tenPlusGrid');
  const tenPlusTitle = document.getElementById('tenPlusTitle');

  function openTenPlus(team) {
    tenPlusGrid.className = `overlay-grid team-${team}`;
    tenPlusTitle.textContent = `Add to ${team === 'red' ? 'Red' : 'Blue'}`;
    tenPlusGrid.innerHTML = '';
    for (let n = 10; n <= 29; n++) {
      const b = document.createElement('button');
      b.type = 'button';
      b.textContent = `+${n}`;
      b.addEventListener('click', () => {
        addScore(team, n);
        closeOverlay(tenPlusOverlay);
      });
      tenPlusGrid.appendChild(b);
    }
    openOverlay(tenPlusOverlay);
  }

  function openOverlay(el)  { el.classList.remove('hidden'); }
  function closeOverlay(el) { el.classList.add('hidden'); }

  document.getElementById('tenPlusCancel').addEventListener('click', () => closeOverlay(tenPlusOverlay));
  tenPlusOverlay.addEventListener('click', (e) => {
    if (e.target === tenPlusOverlay) closeOverlay(tenPlusOverlay);
  });

  // ---------- New game overlay ----------
  const newGameOverlay = document.getElementById('newGameOverlay');
  document.getElementById('newBtn').addEventListener('click', () => openOverlay(newGameOverlay));
  document.getElementById('cancelNew').addEventListener('click', () => closeOverlay(newGameOverlay));
  document.getElementById('confirmNew').addEventListener('click', () => {
    newGame();
    closeOverlay(newGameOverlay);
  });
  newGameOverlay.addEventListener('click', (e) => {
    if (e.target === newGameOverlay) closeOverlay(newGameOverlay);
  });

  // ---------- Undo ----------
  document.getElementById('undoBtn').addEventListener('click', undo);

  // ---------- Keyboard ----------
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      closeOverlay(tenPlusOverlay);
      closeOverlay(newGameOverlay);
    }
  });

  // ---------- Device orientation ----------
  // Rotate the score + button labels based on the physical device orientation
  // (not the browser viewport), so the same side always stays "up" in the user's
  // hand. We use the Screen Orientation API when available and fall back to
  // window.orientation (older iOS) or the default (0).
  function currentOrientationAngle() {
    if (screen && screen.orientation && typeof screen.orientation.angle === 'number') {
      return screen.orientation.angle;
    }
    if (typeof window.orientation === 'number') {
      return ((window.orientation % 360) + 360) % 360;
    }
    return 0;
  }

  function updateOrientation() {
    const angle = currentOrientationAngle();
    // angle is 0, 90, 180, or 270 (the angle the content has been rotated by the OS).
    // To counter-rotate our labels so they stay upright relative to the device,
    // rotate labels by -angle.
    document.body.dataset.rotate = String(angle);
  }

  updateOrientation();
  if (screen && screen.orientation && screen.orientation.addEventListener) {
    screen.orientation.addEventListener('change', updateOrientation);
  } else {
    window.addEventListener('orientationchange', updateOrientation);
  }

  // ---------- Init ----------
  load();
  buildScoreButtons();
  render();
  if (state.winner) showGameOver();

  // ---------- Service Worker ----------
  if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
      navigator.serviceWorker.register('service-worker.js').catch(() => {});
    });
  }
})();
