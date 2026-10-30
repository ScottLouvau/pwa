# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

A standalone Progressive Web App scoreboard for the card game Cribbage. No framework, no build tool — pure HTML/CSS/JS with SVG rendering. Supports 2-3 teams, click-to-score with animated pegs, undo/redo, and new game.

## Structure

```
index.html          — Single page, loads index.css + index.js
index.css           — CSS variables for theming, safe-area insets for PWA
index.js            — All logic: SVG board rendering, state management, event handlers
icons/              — PWA icons (SVG: icon-192.svg, icon-512.svg, board-test.svg)
publish             — Deploys everything to ../publish/cribbage
.claude/claude.md  — Agent behavioral guidance
```

## How It Works

- **State** (`state` object): Tracks teams (each with `score`, `last`, `color`), `undo` array, and `redo` array.
- **Board layout**: Four sides (top/right/bottom/left) with 120 holes each, mapped via `HOLE_RANGES`. Score 1-120 maps to holes via `scoreToHole()`.
- **Rendering**: `renderBoard()` regenerates the full SVG from scratch. `redrawPegs()`/`showScoreIncrease()`/`showScoreDecrease()` do targeted DOM updates after scoring.
- **Scoring**: `POSSIBLE_SCORES` array defines clickable button values. Each team gets a 5x5 grid of +score buttons plus a score total display.
- **Undo/Redo**: Stacks of `{team, last, score}` snapshots. Adding to score clears redo stack.

## Developing

- Open `index.html` directly in a browser — no build step needed.
- Test by opening `index.html` in a browser. Click the colored button grids to score points. Test undo/redo with buttons and keyboard (Backspace = new game).
- Use the `publish` script to deploy: `./publish` copies files to `../publish/cribbage`.
- For PWA testing, serve over HTTP/HTTPS (e.g., `python3 -m http.server`) to enable service worker registration.

## Key Constants

- `BOARD_WIDTH=780`, `BOARD_HEIGHT=380` — viewBox dimensions
- `HOLE_SPACING=13` — pixel distance between holes
- `POSSIBLE_SCORES` — [1..18, 20, 21, 22, 23, 24, 28, 29]
- Color variables in `index.css` `:root` (`--red`, `--blue`, `--white`, etc.)

## Agent Guidance

- Design a pure JavaScript app with minimal dependencies
- Keep code simple and straightforward
- When fixing bugs, make minimal, tactical changes — don't refactor unrelated code
