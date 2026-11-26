// Package C private: time remap + measured geometry of shot 6 `flowcli` (freeze-annotate).
// public/clips/flowcli.mp4 is 2044 x 1620, 173 f; mp4 frame = source frame - 24.
// Measured on the mp4 (frame-difference scan + pixel scan of frame 47):
//   typing  mp4 10-38 (a char pair every 2 f), Enter 40, --watch redraw 46 (8 -> 9 nodes),
//   click picks Backorder 88, drag steps 94, 97, ... 121 (one TUI redraw per 3 f),
//   click on the canvas clears 151.
//   new node "Email buyer": box x 1372-1590, y 1418-1499 (mp4 px), unchanged until 151.

export const CLI = { src: 'clips/flowcli.mp4', w: 2044, h: 1620 } as const;
export const CARD = { w: 966, h: 765 } as const;
export const S = CARD.w / CLI.w; // 0.4726, clip fitted to the card width (zoom 1)

export const FREEZE = 67; // instant cut to slope 0
export const THAW = 118;
export const LAND = 146;
export const SRC_FREEZE = 47;
export const SRC_LAND = 121;

const smootherstep = (t: number) => {
  const x = Math.max(0, Math.min(1, t));
  return x * x * x * (x * (6 * x - 15) + 10);
};

/** first source frame of the unfreeze ramp. mp4 frames 50-87 are pixel-identical (a static
 *  terminal: one pixel differs by 26/255, codec noise) and match the frozen frame 47, so the
 *  ramp starts at 86: the skip over identical frames is invisible, and the ramp's ease-in start
 *  lands on what actually moves (the click on Backorder at src 88, then its drag 94-121) instead
 *  of being spent on a still terminal. Same total compensation as DESIGN.md (src 47 -> 121 in
 *  28 f); the visible drag plays at ~1.3-1.6x, decelerating, instead of ~3x in 11 f. */
export const SRC_THAW_FROM = 86;

/** output -> source frame (DESIGN.md 8.6, adapted as above):
 *  0-20 src 0 (fades in 12-20) | 20-67 1x (src = f - 20) | 67-118 FROZEN on src 47 (51 f) |
 *  118-146 smootherstep 86 -> 121 (ease-in start, click at ~124, drag ~128-146 braking) |
 *  146-156 src 121. */
export const cliSrcAt = (f: number) => {
  if (f < 20) return 0;
  if (f < FREEZE) return f - 20;
  if (f < THAW) return SRC_FREEZE;
  if (f < LAND) return SRC_THAW_FROM + (SRC_LAND - SRC_THAW_FROM) * smootherstep((f - THAW) / (LAND - THAW));
  return SRC_LAND;
};

/** the target: new node "Email buyer", in card px. */
const NODE_SRC = { x0: 1372, y0: 1418, x1: 1590, y1: 1499 };
export const NODE = { x0: NODE_SRC.x0 * S, y0: NODE_SRC.y0 * S, x1: NODE_SRC.x1 * S, y1: NODE_SRC.y1 * S };
export const NODE_C = { x: (NODE.x0 + NODE.x1) / 2, y: (NODE.y0 + NODE.y1) / 2 };
export const PAD = 18; // marker clearance around the node (card px)
/** the undimmed window (node + pad). */
export const HOLE = { x0: NODE.x0 - PAD, y0: NODE.y0 - PAD, x1: NODE.x1 + PAD, y1: NODE.y1 + PAD };

// One-stroke marker superellipse (demo FreezeAnnotateReal circlePath): n = 3, starts upper-left,
// 1.08 turns clockwise, radius tightens 4 % along the stroke so the end does not close.
// Radii = 1.2 x the half-size of the padded box (demo: 1.2 x card half-size).
export const RX = 1.2 * ((NODE.x1 - NODE.x0) / 2 + PAD);
export const RY = 1.2 * ((NODE.y1 - NODE.y0) / 2 + PAD);
export const markerPath = (() => {
  const N = 140;
  const a0 = -2.3;
  const SE = 2 / 3;
  const pts: string[] = [];
  for (let i = 0; i <= N; i++) {
    const u = i / N;
    const a = a0 + u * Math.PI * 2 * 1.08;
    const k = 1 - 0.04 * u;
    const c = Math.cos(a), sn = Math.sin(a);
    const px = Math.sign(c) * Math.pow(Math.abs(c), SE) * RX * k;
    const py = Math.sign(sn) * Math.pow(Math.abs(sn), SE) * RY * k;
    pts.push(`${(NODE_C.x + px).toFixed(1)} ${(NODE_C.y + py).toFixed(1)}`);
  }
  return `M ${pts.join(' L ')}`;
})();

/** point on the (untightened) superellipse at angle a (radians). */
export const markerPoint = (a: number) => {
  const SE = 2 / 3;
  const c = Math.cos(a), sn = Math.sin(a);
  return { x: NODE_C.x + Math.sign(c) * Math.pow(Math.abs(c), SE) * RX, y: NODE_C.y + Math.sign(sn) * Math.pow(Math.abs(sn), SE) * RY };
};
