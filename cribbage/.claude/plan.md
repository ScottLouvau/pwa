# Implementation Plan

### File layout
index.html — single page, semantic markup, PWA meta tags, SVG inlined via JS
index.css — layout + theming via CSS variables, orientation media queries
index.js — state, rendering, event wiring, SW registration
manifest.webmanifest — PWA manifest (name, icons, theme, display: standalone)
service-worker.js — cache-first for static assets
icons/ — 192px and 512px SVG icons, of a zoomed in segment of the cribbage scoring holes and pegs for two players.


### Layout (CSS Grid)
Three rows × three columns:

[  red score  ][     ][  blue score ]
[  red btns   ][board][  blue btns  ]
[         undo / help / new         ]

Portrait: board is tall and narrow; team tracks run vertically (0 at bottom, 121 at top)
Landscape: same as portrait with the scores and button labels rotated
Buttons sized for thumb reach (min 44px), grid gap scales with viewport

Board SVG (generated in JS)
viewBox computed from: numTeams, hole spacing, and 121 positions
Per team: a vertical track of 121 peg holes
Holes grouped in 5s with a small vertical gap between groups
Faint horizontal line at each 5-point boundary
Darker line at position 91 (skunk)
Don't show holes below zero or above 120. 
Pegs: two <circle> elements per team, larger than holes, filled with team color
Current peg at score, trailing peg at previousScore
Hidden when score is 0 or no history
Theming via CSS variables on <svg>: --hole-color, --board-bg, --board-bg-image
Re-rendered whenever state changes (simple; 121 × 2 circles is tiny)


### State model

state = {
	red_total: 94,
	blue_total: 88,
	history = [
		{ team: "red", score: 4 },
		... (latest scoring at end of array)
	]
}

addScore(teamColor, amount) pushes to history, updates score, re-renders, triggers animation
undo() pops most recent entry across any team (single global history stack is cleaner — I'll use that instead of per-team history)
Persist state to localStorage on every change; load on startup

### Animations
CSS class .score-bump applied to the changed score element, removed via animationend — scale + color pulse
Peg movement is immediate (no tween) — keeps code simple; animation on the score number alone is enough emphasis
10+ overlay
Fixed-position overlay, two columns of +10…+29 buttons, Cancel button
One overlay element, populated dynamically with the team's color when opened
Dismissed on button click, Cancel, or backdrop tap

### Undo / New
Inline SVG icons (↶ and ?) to avoid dependencies
Undo removes the last score added from the history and redraws the interface. 
New verifies via modal, then resets the game (both scores zero), no history.

### PWA
service-worker.js precaches the five static files on install, serves from cache on fetch, network fallback for updates
Cache version bumped via constant in the SW file
Manifest: theme color matches board wood tone, standalone display, portrait-primary orientation preference
