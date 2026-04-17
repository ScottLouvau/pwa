(() => {
  'use strict';

  // ---------- State ----------
  const STORAGE_KEY = 'cribbageState';

  const state = {
    red_total: 0,
    blue_total: 0,
    history: [], // [{ team: 'red'|'blue', score: number }]
  };

  function save() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    } catch (e) { /* storage full or disabled */ }
  }

  function load() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return;
      const saved = JSON.parse(raw);
      if (saved && typeof saved === 'object') {
        state.red_total = saved.red_total || 0;
        state.blue_total = saved.blue_total || 0;
        state.history = Array.isArray(saved.history) ? saved.history : [];
      }
    } catch (e) { /* ignore */ }
  }

  function teamLastAdd(team) {
    for (let i = state.history.length - 1; i >= 0; i--) {
      if (state.history[i].team === team) return state.history[i].score;
    }
    return 0;
  }

  function addScore(team, amount) {
    if (team === 'red') state.red_total += amount;
    else state.blue_total += amount;
    state.history.push({ team, score: amount });
    save();
    render();
    bumpScore(team);
  }

  function undo() {
    if (state.history.length === 0) return;
    const last = state.history.pop();
    if (last.team === 'red') state.red_total -= last.score;
    else state.blue_total -= last.score;
    save();
    render();
    bumpScore(last.team);
  }

  function newGame() {
    state.red_total = 0;
    state.blue_total = 0;
    state.history = [];
    save();
    render();
  }

  // ---------- Board geometry ----------
  const HOLES = 121;          // hole indices 0..120
  const HOLE_R = 2.2;
  const HOLE_SP = 6;          // vertical spacing between adjacent holes
  const GROUP_GAP = 3;        // extra vertical gap between groups of 5
  const TRACK_SP = 20;        // horizontal spacing between team tracks
  const PAD_X = 10;
  const PAD_Y = 14;
  const PEG_R = 4.2;
  const SKUNK_AT = 91;        // skunk line at score 91
  const NUM_TEAMS = 2;        // red, blue for v1

  function yFromBottom(i) {
    return PAD_Y + i * HOLE_SP + Math.floor(i / 5) * GROUP_GAP;
  }
  const BOARD_H = yFromBottom(HOLES - 1) + PAD_Y;
  const BOARD_W = PAD_X * 2 + (NUM_TEAMS - 1) * TRACK_SP;

  function holeY(i) {
    return BOARD_H - yFromBottom(i);
  }

  function trackX(teamIdx) {
    const centerX = BOARD_W / 2;
    return centerX + (teamIdx - (NUM_TEAMS - 1) / 2) * TRACK_SP;
  }

  // ---------- Rendering ----------
  const board = document.getElementById('board');
  board.setAttribute('viewBox', `0 0 ${BOARD_W} ${BOARD_H}`);

  function renderBoard() {
    const parts = [];

    // Background
    parts.push(
      `<rect x="0" y="0" width="${BOARD_W}" height="${BOARD_H}" fill="var(--board-bg)"/>`
    );

    // Faint group lines (every 5 holes; 1..24 boundaries)
    for (let k = 1; k <= 24; k++) {
      const y = (holeY(5 * k - 1) + holeY(5 * k)) / 2;
      parts.push(
        `<line x1="2" y1="${y}" x2="${BOARD_W - 2}" y2="${y}" stroke="var(--group-line)" stroke-width="0.6"/>`
      );
    }

    // Skunk line (at 91): line drawn between holes 90 and 91
    const skunkY = (holeY(SKUNK_AT - 1) + holeY(SKUNK_AT)) / 2;
    parts.push(
      `<line x1="2" y1="${skunkY}" x2="${BOARD_W - 2}" y2="${skunkY}" stroke="var(--skunk-line)" stroke-width="1.6"/>`
    );

    // Holes (0..120)
    for (let t = 0; t < NUM_TEAMS; t++) {
      const x = trackX(t);
      for (let i = 0; i < HOLES; i++) {
        parts.push(
          `<circle cx="${x}" cy="${holeY(i)}" r="${HOLE_R}" fill="var(--hole-color)"/>`
        );
      }
    }

    // Pegs (current + previous for each team)
    const teams = [
      { idx: 0, color: 'var(--red)',  total: state.red_total,  last: teamLastAdd('red')  },
      { idx: 1, color: 'var(--blue)', total: state.blue_total, last: teamLastAdd('blue') },
    ];
    for (const team of teams) {
      if (team.total <= 0) continue; // hide when score is 0 or no history

      const x = trackX(team.idx);
      const currentPos = Math.min(team.total, HOLES - 1);

      // Trailing peg (at previous score) — only if there's a prior add
      if (team.last > 0) {
        const prevPos = Math.max(0, Math.min(team.total - team.last, HOLES - 1));
        parts.push(
          `<circle cx="${x}" cy="${holeY(prevPos)}" r="${PEG_R}" fill="${team.color}" opacity="0.55" stroke="rgba(0,0,0,0.3)" stroke-width="0.4"/>`
        );
      }

      // Current peg
      parts.push(
        `<circle cx="${x}" cy="${holeY(currentPos)}" r="${PEG_R}" fill="${team.color}" stroke="rgba(0,0,0,0.4)" stroke-width="0.5"/>`
      );
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
    // Force reflow so the animation can replay on rapid taps.
    void el.offsetWidth;
    el.classList.add('bump');
  }

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

  function openOverlay(el) { el.classList.remove('hidden'); }
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

  // ---------- Keyboard: Escape closes overlays ----------
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      closeOverlay(tenPlusOverlay);
      closeOverlay(newGameOverlay);
    }
  });

  // ---------- Init ----------
  load();
  buildScoreButtons();
  render();

  // ---------- PWA service worker ----------
  if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
      navigator.serviceWorker.register('service-worker.js').catch(() => { /* ignore */ });
    });
  }
})();
