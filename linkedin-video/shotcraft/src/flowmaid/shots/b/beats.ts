// Package B beat sheet (shot-local frames). The shot components AND their .cues.ts files read
// these constants, so the cue sheet can never drift from what is rendered. Data + pure math only.
import { bezier, ease } from '../../tokens';

// ── shot 3 `speed` (113 f) ────────────────────────────────────────────────
// Fixer round 1 (review: the lock + footnote held only ~10-15 f before the outgoing wipe). The
// shot is 8 f longer (taken from the outro hold) and the run is 6-7 f earlier, so the locked
// composition (~113x + claim, footnote already in) is complete at 81 and holds 32 f fully
// still before the wipe at 113 (card gauge-readout-moves: "停位后 >=30f 全锁死"; R1).
export const SPEED_BEATS = {
  paneIn: 4, // tape pane rules start drawing (inside the 16 f wipe)
  tapeIn: 10, // 10-20 tape content fades in (pointer line draws 11-23)
  pointerIn: 11,
  footAt: 16, // 16-28 footnote fades in with the readout (it qualifies the metric for the whole shot)
  creep: 12, // 12-28 creep 1.00 -> 1.12 (swift); parked on MERMAID.JS ~12 f after the tape is in
  breakAt: 28, // 28-34 MERMAID.JS limit line splits (halves slide 8 px apart) and fades
  breakDur: 6,
  antAt: 32, // 32-37 anticipation 1.12 -> 0.95
  sprintAt: 37, // 37-59 sprint 0.95 -> 122 (bezier .55,0,.25,1), vertical speed blur on the tape
  sprintDur: 22,
  brakeAt: 59, // 59-75 spring brake 122 -> (112.3) -> 113, one visible swing (residual < 0.05 px at 75)
  lockAt: 75, // constant 113 from here; readout becomes "~113x"  (THE hit)
  lockFx: 6, // 75-81 the "~" rises and the FLOWMAID marker comes to full strength (exactly done at 81)
  claimIn: 71, // 71-81 "FASTER THAN MERMAID.JS" mask rise
  claimDur: 10,
  stillAt: 81, // from here every pixel of the shot is constant (32 f to the cut at 113)
} as const;

// ── shot 4 `types` (110 f) ────────────────────────────────────────────────
// Fixer round 1 (review: the materialised 12/12 grid held only ~20 f before the outgoing wipe,
// the faint blueprint ~2 s). The blueprint stage is compressed (6-42 instead of 10-64, same
// group order and stagger shape, ~0.7x), so the scan runs 48-78, the counter lands on 12/12 (and
// turns accent) at 78 and the real renders hold 32 f (30 f fully still after the scan line fades
// at 80) before the wipe at 110.
export const TYPES_BEATS = {
  frameAt: 6, // outer frame 6-30
  frameDur: 24,
  colAt: 9, // column rules 9 + 2c, 20 f
  colStep: 2,
  rowAt: 11, // row rules 11 + 2r, 20 f
  rowStep: 2,
  gridDur: 20,
  headAt: 14, // cell header rules 14 + 1.5i, 12 f (labels fade in 2 f after their rule starts)
  headStep: 1.5,
  headDur: 12,
  wireAt: 12, // wire diagrams 12 + 1.1i, 18 f each (last ends ~42)
  wireStep: 1.1,
  wireDur: 18,
  breathAt: 42, // blueprint complete; 42-48 breath (card: ~6 f between draw-on and scan)
  scanAt: 48, // 48-78 scan top -> bottom (bezier .55,0,.25,1, card value)
  scanDur: 30,
  lineIn: 46, // scan line fades in 46-50, out 76-80
  lineOut: 76,
} as const;

/** the scan easing (card: bezier(0.55, 0, 0.25, 1)); one value drives clip + line. */
export const SCAN_EASE = bezier(0.55, 0, 0.25, 1);
export const scanAt = (frame: number) => SCAN_EASE((frame - TYPES_BEATS.scanAt) / TYPES_BEATS.scanDur);

/** local frame (fractional) at which a row counts as materialised: the scan edge has passed 90 %
 *  of the row (≈ the bottom of its diagram box). The last row lands at ≈ 72.4, so the counter's
 *  5 f roll reads 12/12 at local 78, when the eased scan itself finishes. */
export const ROW_THRESHOLD = 0.9;
export const ROW_DONE: number[] = [1, 2, 3, 4].map((k) => {
  const target = (k - 1 + ROW_THRESHOLD) / 4;
  let lo: number = TYPES_BEATS.scanAt;
  let hi: number = TYPES_BEATS.scanAt + TYPES_BEATS.scanDur;
  for (let i = 0; i < 50; i++) {
    const mid = (lo + hi) / 2;
    if (scanAt(mid) < target - 1e-9) lo = mid;
    else hi = mid;
  }
  return hi;
});

/** counter value (fractional, 0..12) at a local frame: each row adds 3 over a 5 f roll. */
export const counterAt = (frame: number) => ROW_DONE.reduce((n, at) => n + 3 * ease.inout((frame - at) / 5), 0);
/** the counter's last roll has landed on 12/12 from this integer frame on; it turns accent on the
 *  same frame (so the colour change marks the settled reading, not a digit mid-roll). */
export const COUNTER_DONE = (() => {
  let f = TYPES_BEATS.scanAt;
  while (counterAt(f) < 12 - 1e-6) f++;
  return f;
})();

/** first frame on which the blueprint is clearly visible: the outer frame (perimeter ~3576 px,
 *  line2 1 px on the canvas, revealed by the incoming wipe from the left) has drawn >= 150 px. */
const DRAW_EASE = bezier(0.4, 0, 0.3, 1); // card line-draw easing (same as Types.tsx)
export const blueprintDraw = (frame: number, start: number, dur: number) => DRAW_EASE(Math.min(1, Math.max(0, (frame - start) / dur)));
export const BLUEPRINT_VISIBLE = (() => {
  const perimeter = 2 * (968 + 820);
  let f = TYPES_BEATS.frameAt;
  while (blueprintDraw(f, TYPES_BEATS.frameAt, TYPES_BEATS.frameDur) * perimeter < 150) f++;
  return f;
})();
