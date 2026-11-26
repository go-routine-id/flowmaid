// Package C private: camera model of shot 5 `browser` (crash-zoom-punch, bounce variant).
// Pure functions of the shot-local frame, so the motion-blur sub-frames can sample it at
// fractional times. All positions are SOURCE px of public/clips/web.mp4 (1590 x 1064);
// zoom 1 = the whole clip fitted to the 966 px card (ClipCard convention).
import { clipToCard, type ClipCamera } from '../../lib/ClipCard';
import { EASE, mix, ramp } from '../../lib/Polish';

export const WEB = { src: 'clips/web.mp4', w: 1590, h: 1064 } as const;
export const CARD = { w: 966, h: 646 } as const;

// ── beats (shot-local) ── fixer round 1: the wide view is established for 30 f after the wipe
// (card crash-zoom-punch: "前 hold >=30f 建立全景"), so everything from the anticipation on runs
// 12 f later than in the first cut; the 12 f come out of the post-release hold.
export const CORNERS_AT = 22; // 22-38 the accent L-corners contract onto the drag region
export const CREEP_FROM = 16; // 16-46 creep 1 -> 1.03
export const ANT = 46; // 46-52 anticipation (x0.97)
export const PUSH = 52; // crash starts
export const HIT = 58; // crash lands (6 f); src 52 = the cursor presses Auth? on this frame
export const SRC_AT_HIT = 52;
const SRC_OFFSET = SRC_AT_HIT - HIT; // src = local - 6
/** web.mp4 frames 118-134 are identical inside the close-up (after the release at 117); at 135 the
 *  cursor starts to leave and at 153 the toolbar hides. Capping at 134 keeps everything from
 *  local 140 on (the knob slide 141-150 included) still. */
export const SRC_CAP = 134;

/** output -> source frame (spec: min(155, f + 6) with the crash at 40; shifted with the beats). */
export const webSrcFrame = (f: number) => Math.min(SRC_CAP, Math.max(0, f + SRC_OFFSET));
/** shot-local frame on which source frame `src` is shown (for cues / notes). */
export const localOfSrc = (src: number) => src - SRC_OFFSET;
// Close-up zoom. Spec: 2.13 at centre (1080, 320). At that framing the left edge cuts the
// code-pane border and the collapse tab (src x 707-746) and the bottom edge touches the
// 401 / Cache boxes (bottom at src y 569 vs edge 570). 2.06 with the left edge pinned at
// src x 750 and centre y 332 keeps the whole drag region (Request .. Cache, src x 774-1401,
// y 98-569) inside with ~25 src px of air, and the overshoot peak (x1.03 = 2.12) stays
// under the 2.13 cap (on-screen source scale: rest 1.25, peak 1.29, creep end 1.26).
export const REST = 2.06;
export const PEAK = REST * 1.03;
const LEFT_EDGE = 750; // src x of the close-up's left edge (code pane + tab stay out)
const FOCUS_Y = 332;
const WIDE = { cx: WEB.w / 2, cy: WEB.h / 2 };

/** what the corners lock onto (src px): the close-up's own frame inset ~8 src px, which holds
 *  the whole drag region (Request, Auth? over its full travel, 401 Denied, Cache). With the
 *  corners 14 src px outside it they sit ~10 card px inside every edge after the crash
 *  (viewfinder), instead of hugging three edges and floating on the fourth. */
export const TARGET = { x0: 772, y0: 96, x1: 1500, y1: 568 } as const;

const clampMin1 = (z: number) => Math.max(1, z);

/** wide framing zoom: 16-46 creep 1 -> 1.03, 46-52 anticipation x0.97 (back to ~1.0).
 *  Spec says creep 1.02 and anticipation x0.96, which dips below 1 and would show the card
 *  background around the clip; 3 % / 3 % keeps the card covered (card: creep 2-3 %,
 *  anticipation 3-4 %). */
export const wideZoom = (f: number) =>
  clampMin1(mix(1, 1.03, ramp(f, CREEP_FROM, ANT - CREEP_FROM, EASE.smooth)) * mix(1, 0.97, ramp(f, ANT, PUSH - ANT, EASE.smooth)));

/** close-up creep from the landing to the release (x1.01), then a settled hold. */
export const RELEASE = localOfSrc(117); // the node toolbar pops back (local 123)
const closeCreep = (f: number) => 1 + 0.01 * ramp(f, HIT + 1, RELEASE - HIT - 1, EASE.smooth);

/** focus point of the close-up: left edge pinned at LEFT_EDGE while the creep zooms. */
const focusAt = (f: number) => ({ x: LEFT_EDGE + WEB.w / (2 * REST * closeCreep(f)), y: FOCUS_Y });

const base = CARD.w / WEB.w;

export const camAt = (f: number): ClipCamera => {
  if (f < PUSH) return { zoom: wideZoom(f), cx: WIDE.cx, cy: WIDE.cy };
  if (f < HIT) {
    // 6 f strong ease-in in log space; the focus point's on-card position moves monotonically
    // from its wide position to the card centre with the same curve (demo camAt).
    const p = ramp(f, PUSH, HIT - PUSH, EASE.exit);
    const z0 = wideZoom(PUSH);
    const zoom = Math.exp(mix(Math.log(z0), Math.log(PEAK), p));
    const F = focusAt(HIT);
    const wide = clipToCard(F, { srcW: WEB.w, srcH: WEB.h, width: CARD.w, height: CARD.h, camera: { zoom: z0, cx: WIDE.cx, cy: WIDE.cy } });
    const sx = mix(wide.x, CARD.w / 2, p);
    const sy = mix(wide.y, CARD.h / 2, p);
    const s = base * zoom;
    return { zoom, cx: F.x - (sx - CARD.w / 2) / s, cy: F.y - (sy - CARD.h / 2) / s };
  }
  // damped-cosine settle (demo: tau 2.2 f, half-period 5.2 f): one visible rebound, then dry
  const t = f - HIT;
  const zoom = (REST + (PEAK - REST) * Math.exp(-t / 2.2) * Math.cos((Math.PI * t) / 5.2)) * closeCreep(f);
  const F = focusAt(f);
  return { zoom, cx: F.x, cy: F.y };
};

/** card-px mapping of a source point under the camera at (fractional) frame f. */
export const toCard = (pt: { x: number; y: number }, f: number) =>
  clipToCard(pt, { srcW: WEB.w, srcH: WEB.h, width: CARD.w, height: CARD.h, camera: camAt(f) });

/** largest on-card displacement of the card's corners/centre between frames a and b. */
export const cardDisplacement = (a: number, b: number) => {
  const A = toCard({ x: 0, y: 0 }, a); // gives tx, ty, scale of camera a
  let m = 0;
  for (const [x, y] of [[0, 0], [CARD.w, 0], [0, CARD.h], [CARD.w, CARD.h], [CARD.w / 2, CARD.h / 2]]) {
    const src = { x: (x - A.x) / A.scale, y: (y - A.y) / A.scale };
    const B = toCard(src, b);
    m = Math.max(m, Math.hypot(B.x - x, B.y - y));
  }
  return m;
};

// Time-sampled motion blur (demo CrashZoomReal): ~200 deg shutter, equal-weight layers of the
// same video frame. Echo spacing is kept <= 6 px measured at the fastest-moving card point
// (n - 1 intervals, so n = ceil(d / 6) + 1, capped at 48), and each layer gets a smear of
// ~0.6 x the spacing (<= 4 px) so the echoes merge into a streak: the playground's code text
// is only ~9 px tall in the wide view, so even 6 px echoes read as a strobe without it.
// Only inside the crash (PUSH+1..HIT).
export const SHUTTER = 0.55;
export const blurSamplesAt = (f: number) => {
  if (f <= PUSH || f > HIT) return 1;
  const d = cardDisplacement(f, f - SHUTTER);
  if (d < 4) return 1; // sub-4 px: a single sharp layer reads better than a double image
  return Math.max(1, Math.min(48, Math.ceil(d / 6) + 1));
};
export const blurSmearAt = (f: number, n: number) => {
  if (n <= 1) return 0;
  const d = cardDisplacement(f, f - SHUTTER);
  const fast = Math.min(1, Math.max(0, (d - 6) / 24));
  return Math.min(4, (d / (n - 1)) * 0.6 * fast);
};
