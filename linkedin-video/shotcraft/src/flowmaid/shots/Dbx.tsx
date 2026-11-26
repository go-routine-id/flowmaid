// DbxShot — shot 7 `dbx` (abs 757-907, 150 f, Ink). Package D.
// Card: beat-cut-moves · style paparazzi-flash (references/shots/rhythm/beat-cut-moves.md,
// demo demos/rhythm/beat-cut-moves/PaparazziFlash.tsx), re-skinned to flowmaid-web Ink.
//
//   0-30    LIVE: dbx explorer, src = floor(local x 0.6) (mp4 f0..17), slow push 1 -> 1.02 (smooth)
//   22-30   focus corners contract 1.3x -> the next crop (whole window); accent lock from 27
//   30      FLASH 1 -> photo B: mp4 f24 frozen (4-table ERD + "ERD generated" toast), crop 1.0
//   44-52   corners contract onto the ERD panel of photo B; lock 49
//   52      FLASH 2 -> photo C: mp4 f62 frozen (zoomed ERD: orders, payments, shipments), crop 1.17
//   62-70   corners contract onto orders + its connectors in photo C; lock 67
//   70      FLASH 3 -> photo D: mp4 f110 frozen (orders highlighted), crop 1.9 on orders + connectors
//   76-140  photo D push 1 -> 1.015 (smooth); 80-98 HUD "LAYOUT  flowmaid er::scene" draws + rises
//   140-150 fully static (knob slides to PAPER over abs 890-899; this last frame is held under the circle)
//
// Hard cuts only (card pitfall): every change of mp4 frame happens on a flash frame, the clip's own
// cuts at mp4 f18 / f30 are never played across (live uses 0..17, photos are single frozen frames).
// Kept card parameters: flash 30/52/70 (22 f -> 18 f), 0.95 -> 0 over 4 f ease-out, +/-2 px seeded jitter
// on the flash frame, shutter settle 1.03 -> 1 and -16 px -> 0 over 6 f ease-out, focus lead 8 f
// (1.3x -> target, lock 3 f before), photos desaturated/contrasted, hold after the 3rd flash 80 f with
// only a 1 -> 1.015 push. Adapted (DESIGN.md section 10): the flash is PANE-BODY level in INK.ink, not a
// full-frame white flash (R4), no grain / vignette / stage light / other cameras' pops / camera HUD.
//
// Crops were measured on the frozen mp4 frames (ffmpeg-extracted PNGs; ink = max channel > 70, plus a
// faint-pixel pass for the top edge of D):
//  - C (f62): text rows 94-128 and 136-183 start at x 40, tables span x 70..1931 (shipments' right
//    border 1926-1931), the bottom hint row is y 1292-1328 with a word gap at x 1912-1943 and the ERD
//    frame line at y 1354. Crop x 25.5..1940, y 56..1340: no row or word is cut (the hint row ends on
//    its "·" before "offset").
//  - D (f110): orders x 135..1002 / y 288..590, "order_id" label to x 1181, shipments' left border
//    at x 1258 (from y 766), payments' left border at x 1280; the dbx header row ends at y 128 and the
//    "[ ERD: main ]" tab row is y 136-183 from x 40, then rows 184-235 are empty until the ERD frame
//    title (y 236). Crop x 28..1207, y 200..991 at zoom 1.9 (fixer round 1: top moved down from 131.5
//    so the tab row and its map-emoji glyph, ~35 px at this zoom, stay out of the 80 f hold; BRIEF
//    "No emoji"); the push to 1.9285 keeps the top edge and moves the sides in to x 36.8..1190 and the
//    bottom to 979: only the ERD frame's 1 px left border crosses an edge, no text row is cut, and
//    payments sits just outside the right edge.
//    A 1.4-1.5 crop that keeps orders AND payments (review suggestion) is not possible without cutting
//    text: the two tables span src x 135..2060 (1925 px), a 1.45 crop shows 1545 px. The hold stays
//    the close-up of orders + its two relationship connectors (DESIGN.md 8.7, deviation in section 10).
import React from 'react';
import { useCurrentFrame } from 'remotion';
import { FONT, INK, TYPE, ease, prog } from '../tokens';
import { Pane, PANE_HEADER_H, SceneLabel } from '../lib/Pane';
import { ClipCard, clipToCard } from '../lib/ClipCard';
import { EASE, mix, ramp } from '../lib/Polish';
import { Brackets, scaleRect, type Rect } from './d/Brackets';
import { MaskRise } from './d/MaskRise';

const p = INK;

// ── timing (shot-local) ──
const FLASHES = [30, 52, 70] as const;
const SETTLE = 6; // shutter settle after a cut
const DECAY = 4; // flash decay
const AF = 8; // focus-corner lead before each flash
const LOCK = 3; // corners turn accent this many frames before the flash
const PUSH_FROM = FLASHES[2] + SETTLE; // 76
const PUSH_DUR = 64; // -> 140, then static to the end
const HUD_AT = 84;

// ── layout (canvas-local) ──
const PANE = { x: 56, y: 120, w: 968, h: 698 } as const; // body 966 x 649
const CARD = { w: 966, h: 648 } as const;
const SRC = { w: 2240, h: 1504 } as const;
const HUD = { top: 836, h: 58 } as const; // rules at y 836 and 894

// ── views ──
type View = { src: number; zoom: number; cx: number; cy: number };
const VIEW_B: View = { src: 24, zoom: 1, cx: SRC.w / 2, cy: SRC.h / 2 };
const VIEW_C: View = { src: 62, zoom: 1.17, cx: 982.75, cy: 698 };
// D: left edge x 28, top edge y 200 (inside the empty rows 184-235 between the "[ ERD: main ]" tab row
// and the ERD frame title); the hold push keeps that top edge fixed (cy follows the zoom).
const D_TOP = 200;
const dCy = (zoom: number) => D_TOP + CARD.h / (2 * (CARD.w / SRC.w) * zoom);
const VIEW_D: View = { src: 110, zoom: 1.9, cx: 28 + SRC.w / (2 * 1.9), cy: dCy(1.9) };
const PHOTOS = [VIEW_B, VIEW_C, VIEW_D];

const cardOpts = (v: { zoom: number; cx: number; cy: number }) => ({
  srcW: SRC.w, srcH: SRC.h, width: CARD.w, height: CARD.h, camera: { zoom: v.zoom, cx: v.cx, cy: v.cy },
});
/** source-px rect -> card-px rect under a view */
const srcRectToCard = (r: Rect, v: View): Rect => {
  const a = clipToCard({ x: r.x, y: r.y }, cardOpts(v));
  const b = clipToCard({ x: r.x + r.w, y: r.y + r.h }, cardOpts(v));
  return { x: a.x, y: a.y, w: b.x - a.x, h: b.y - a.y };
};
const INSET = 10;
const clampToCard = (r: Rect): Rect => {
  const x0 = Math.max(INSET, r.x);
  const y0 = Math.max(INSET, r.y);
  const x1 = Math.min(CARD.w - INSET, r.x + r.w);
  const y1 = Math.min(CARD.h - INSET, r.y + r.h);
  return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
};

// Focus targets: "the next photo", drawn in the CURRENT view's card px.
//  0: live -> B (crop 1.0 = whole window): the whole card, inset 14.
//  1: B -> C: the ERD panel of mp4 f24 (frame lines x 741..2190, y ~254..1355), which C enlarges.
//  2: C -> D: D's crop expressed relative to the orders table, mapped onto orders in mp4 f62
//     (orders x 70..829 / title row y 288 there vs x 135..1002 / y 288 in f110, scale 0.875):
//     D's crop x 28..1207 / y 200..991 maps to src rect x -24..1008, y 211..903 (clamped to the card).
const FOCUS: Rect[] = [
  { x: 14, y: 14, w: CARD.w - 28, h: CARD.h - 28 },
  clampToCard(srcRectToCard({ x: 735, y: 236, w: 1461, h: 1126 }, VIEW_B)),
  clampToCard(srcRectToCard({ x: -24, y: 211, w: 1032, h: 692 }, VIEW_C)),
];

// seeded hash (deterministic jitter)
const hash = (i: number) => {
  const s = Math.sin(i * 127.3 + 11.7) * 43758.5453;
  return s - Math.floor(s);
};

export const DbxShot: React.FC = () => {
  const frame = useCurrentFrame();

  let seg = -1; // -1 live, 0 photo B, 1 photo C, 2 photo D
  for (let i = 0; i < FLASHES.length; i++) if (frame >= FLASHES[i]) seg = i;

  // camera
  const liveZoom = 1 + 0.02 * ramp(frame, 0, FLASHES[0], EASE.smooth);
  const holdPush = 1 + 0.015 * ramp(frame, PUSH_FROM, PUSH_DUR, EASE.smooth);
  const view: View =
    seg === -1
      ? { src: 0, zoom: liveZoom, cx: SRC.w / 2, cy: SRC.h / 2 }
      : seg === 2
        ? { ...VIEW_D, zoom: VIEW_D.zoom * holdPush, cy: dCy(VIEW_D.zoom * holdPush) }
        : PHOTOS[seg];

  // shutter settle (1.03 -> 1, -16 px -> 0, 6 f ease-out), jitter on the flash frame only
  let settleScale = 1;
  let settleY = 0;
  let jx = 0;
  let jy = 0;
  if (seg >= 0) {
    const t = 1 - ramp(frame, FLASHES[seg], SETTLE, EASE.out);
    settleScale = 1 + 0.03 * t;
    settleY = -16 * t;
    if (frame === FLASHES[seg]) {
      jx = 2 * (hash(frame * 7 + 1) * 2 - 1);
      jy = 2 * (hash(frame * 13 + 5) * 2 - 1);
    }
  }
  let flash = 0;
  for (const f of FLASHES) if (frame >= f && frame <= f + DECAY) flash = Math.max(flash, 0.95 * (1 - ramp(frame, f, DECAY, EASE.out)));

  // focus corners for the NEXT flash
  let focus: React.ReactNode = null;
  const next = seg + 1;
  if (next < FLASHES.length) {
    const fNext = FLASHES[next];
    const start = fNext - AF;
    if (frame >= start && frame < fNext) {
      const k = mix(1.3, 1, ramp(frame, start, 6, EASE.snappy));
      const locked = frame >= fNext - LOCK;
      focus = (
        <svg width={CARD.w} height={CARD.h} style={{ position: 'absolute', left: 0, top: 0, overflow: 'visible' }}>
          <Brackets r={scaleRect(FOCUS[next], k)} color={locked ? p.accent : p.muted} width={2} arm={36} opacity={locked ? 1 : 0.9} />
        </svg>
      );
    }
  }

  // clip source frame: live plays mp4 f0..17 at 0.6x; every photo is ONE frozen frame
  const map = (f: number) => {
    if (f < FLASHES[0]) return Math.min(17, Math.floor(f * 0.6));
    if (f < FLASHES[1]) return VIEW_B.src;
    if (f < FLASHES[2]) return VIEW_C.src;
    return VIEW_D.src;
  };

  const cardIn = ease.quiet(prog(frame, 4, 10));

  // HUD row under the pane
  const hudRule = ease.expo(prog(frame, HUD_AT - 4, 14));
  const hudKey = ease.expo(prog(frame, HUD_AT, 14));
  const hudVal = ease.expo(prog(frame, HUD_AT + 3, 14));

  return (
    <div style={{ position: 'absolute', inset: 0 }}>
      <SceneLabel
        p={p}
        frame={frame}
        n="06"
        label="DBX"
        right={
          <span style={{ fontFamily: FONT.mono, fontWeight: 500, fontSize: 22, lineHeight: 1, letterSpacing: TYPE.labelTracking, textTransform: 'uppercase', color: p.muted }}>
            Terminal database client
          </span>
        }
      />

      <Pane p={p} frame={frame} x={PANE.x} y={PANE.y} w={PANE.w} h={PANE.h} start={2} dur={22} title="DBX — SHOP-DEMO" tag="SQLITE" fill="panel">
        <div style={{ position: 'absolute', left: 0, top: 0, width: CARD.w, height: CARD.h, overflow: 'hidden', opacity: cardIn }}>
          <div
            style={{
              position: 'absolute', inset: 0, transformOrigin: '50% 0%',
              transform: `translate(${jx.toFixed(2)}px, ${(jy + settleY).toFixed(2)}px) scale(${settleScale.toFixed(5)})`,
              // a frozen frame reads as a photograph: slightly desaturated, a touch more contrast
              filter: seg >= 0 ? 'saturate(0.9) contrast(1.05)' : undefined,
            }}
          >
            <ClipCard src="clips/dbx.mp4" srcW={SRC.w} srcH={SRC.h} width={CARD.w} height={CARD.h} camera={{ zoom: view.zoom, cx: view.cx, cy: view.cy }} map={map} background={p.canvas} />
          </div>
          {focus}
          {flash > 0.004 ? <div style={{ position: 'absolute', inset: 0, background: p.ink, opacity: flash }} /> : null}
        </div>
      </Pane>

      {/* HUD: LAYOUT  flowmaid er::scene */}
      <div style={{ position: 'absolute', left: PANE.x, width: PANE.w, top: HUD.top, height: HUD.h }}>
        <div style={{ position: 'absolute', left: 0, right: 0, top: 0, height: 1, background: p.line2, transformOrigin: 'left', transform: `scaleX(${hudRule})` }} />
        <div style={{ position: 'absolute', left: 0, right: 0, bottom: 0, height: 1, background: p.line2, transformOrigin: 'left', transform: `scaleX(${hudRule})` }} />
        <div style={{ position: 'absolute', left: 20, top: 0, bottom: 0, display: 'flex', alignItems: 'center' }}>
          <MaskRise t={hudKey} height={24}>
            <span style={{ width: 10, height: 10, background: p.accent, marginRight: 16 }} />
            <span style={{ fontFamily: FONT.mono, fontWeight: 500, fontSize: 22, lineHeight: 1, letterSpacing: TYPE.eyebrowTracking, textTransform: 'uppercase', color: p.muted }}>
              Layout
            </span>
          </MaskRise>
        </div>
        <div style={{ position: 'absolute', left: 214, top: 0, bottom: 0, display: 'flex', alignItems: 'center' }}>
          <MaskRise t={hudVal} height={36}>
            <span style={{ fontFamily: FONT.mono, fontWeight: 500, fontSize: 30, lineHeight: 1.2, color: p.ink, whiteSpace: 'pre' }}>flowmaid er::scene</span>
          </MaskRise>
        </div>
      </div>
    </div>
  );
};

/** pane-body box in canvas px (for overlays / QA). */
export const DBX_BODY = { x: PANE.x + 1, y: PANE.y + PANE_HEADER_H, w: CARD.w, h: CARD.h } as const;
