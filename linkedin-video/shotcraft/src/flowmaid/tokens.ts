// flowmaid promo — design tokens (single source for colour, type, layout, motion).
// Source of truth: ../fmweb/index.html (flowmaid-web playground CSS vars) and the
// approved style frames ../fw/style.html + ../fw/fx.html. Do NOT invent new colours:
// every scene, caption, transition and annotation reads from PAPER / INK below.
//
// Rules from the brief: square corners, 1px rules, NO shadows, NO gradients
// ("structure comes from lines"), no glow, no emoji.
import type { CSSProperties } from 'react';
import { cancelRender, continueRender, delayRender, staticFile } from 'remotion';

// ───────────────────────── colour modes ─────────────────────────

export type Mode = 'paper' | 'ink';

export type Palette = {
  bg: string; panel: string; mass: string; ink: string; dim: string; muted: string;
  line: string; line2: string; accent: string; accentInk: string; accentSoft: string;
  canvas: string; dot: string;
};

export const PAPER: Palette = {
  bg: '#efece5', panel: '#e7e3da', mass: '#ddd8cc', ink: '#15140f', dim: '#55534a', muted: '#86826f',
  line: '#e2ded4', line2: '#cbc5b6', accent: '#8839ef', accentInk: '#ffffff', accentSoft: 'rgba(136,57,239,.10)',
  canvas: '#f5f3ee', dot: '#d3cdbf',
};

export const INK: Palette = {
  bg: '#17171a', panel: '#1e1e22', mass: '#26262b', ink: '#eae8ee', dim: '#cdccd2', muted: '#908f96',
  line: '#26262b', line2: '#35353d', accent: '#cba6f7', accentInk: '#17171a', accentSoft: 'rgba(203,166,247,.12)',
  canvas: '#141417', dot: '#2b2b31',
};

export const palette = (mode: Mode): Palette => (mode === 'ink' ? INK : PAPER);

// Playground syntax colours (paper). Used for hero.mmd and `cargo add` code.
export const SYNTAX = {
  header: '#8839ef', // flowchart / graph header keyword
  kw: '#1d5ad6', // subgraph / end
  dir: '#0e6f7c', // TD / LR
  str: '#2d7a3e', // edge label text (|yes|)
  text: '#6b5300', // node text inside brackets
  op: '#b54a12', // arrows -->
  pu: '#55534a', // punctuation ([ ]) { } |
} as const;

// ───────────────────────── type ─────────────────────────

export const FONT = {
  display: 'Archivo, Inter, system-ui, sans-serif', // 900, letter-spacing -.045em
  body: 'Inter, system-ui, sans-serif', // 400/500/600
  mono: '"JetBrains Mono", ui-monospace, monospace', // labels uppercase .07–.16em
} as const;

export const TYPE = {
  displayTracking: '-0.045em',
  labelTracking: '0.07em',
  eyebrowTracking: '0.16em',
} as const;

// Local font files (copied into public/fonts; Google Fonts is not reachable from the
// render box). Archivo 900/800 = the Google Fonts TTFs; Inter + JetBrains Mono = the
// same families as the playground. Loaded once per bundle, render waits for them.
const FONT_FILES: { family: string; file: string; weight: string }[] = [
  { family: 'Archivo', file: 'fonts/Archivo-900.ttf', weight: '900' },
  { family: 'Archivo', file: 'fonts/Archivo-800.ttf', weight: '800' },
  { family: 'Inter', file: 'fonts/Inter-Regular.otf', weight: '400' },
  { family: 'Inter', file: 'fonts/Inter-Medium.otf', weight: '500' },
  { family: 'Inter', file: 'fonts/Inter-SemiBold.otf', weight: '600' },
  { family: 'JetBrains Mono', file: 'fonts/JetBrainsMono-Regular.ttf', weight: '400' },
  { family: 'JetBrains Mono', file: 'fonts/JetBrainsMono-Medium.ttf', weight: '500' },
  { family: 'JetBrains Mono', file: 'fonts/JetBrainsMono-SemiBold.ttf', weight: '600' },
];

let fontsRequested = false;
export const loadFlowmaidFonts = () => {
  if (fontsRequested || typeof document === 'undefined') return;
  fontsRequested = true;
  // Generous timeout: with 4 render tabs + OffthreadVideo extraction, one tab's font fetch was
  // measured to take > 28 s once (integration render 1 failed on the 30 s default).
  const handle = delayRender('flowmaid fonts', { timeoutInMilliseconds: 120000 });
  Promise.all(
    FONT_FILES.map((f) =>
      new FontFace(f.family, `url(${staticFile(f.file)})`, { weight: f.weight, style: 'normal' })
        .load()
        .then((face) => {
          document.fonts.add(face);
        }),
    ),
  )
    .then(() => continueRender(handle))
    // fail the render loudly rather than silently rendering fallback fonts
    .catch((err) => cancelRender(err));
};

// ───────────────────────── layout (1080 × 1350, 4:5) ─────────────────────────

export const FRAME = { w: 1080, h: 1350 } as const;
export const G = 56; // side gutter (playground --G)
export const HEADER_H = 96;
export const FOOTER_H = 54;

// The dotted canvas between header and footer. Shot components draw in CANVAS-LOCAL
// coordinates: (0,0) = top-left of the canvas = frame (0, 96).
export const CANVAS = { top: HEADER_H, w: 1080, h: FRAME.h - HEADER_H - FOOTER_H } as const; // h = 1200

// Safe content box for shots (canvas-local). The caption band sits below it.
export const SAFE = { x: G, w: FRAME.w - 2 * G, top: 40, bottom: 940 } as const; // frame y 136..1036
// Caption band (canvas-local): bottom edge at canvas 1100 = frame 1196, above LinkedIn's
// mobile overlay (frame y > 1215 = canvas y > 1119). Nothing important below canvas 1100.
export const CAPTION_BAND = { top: 954, bottom: 1100 } as const;
export const LINKEDIN_OVERLAY_Y = 1215; // frame coords

// Dotted whiteboard: 28px grid, dot 1.6px (fx.html), offset 14px.
export const DOT_GRID = 28;
export const dotBackground = (p: Palette): CSSProperties => ({
  backgroundColor: p.canvas,
  backgroundImage: `radial-gradient(${p.dot} 1.6px, transparent 1.7px)`,
  backgroundSize: `${DOT_GRID}px ${DOT_GRID}px`,
  backgroundPosition: '14px 14px',
});
// NOTE: the radial-gradient above is the playground's own dot texture (a 1.6px dot
// tile), not a decorative gradient — it is the one allowed use of a CSS gradient.

// Header geometry (frame coords) — the S01 overlay lands its wordmark exactly here.
export const HEADER = {
  wordmarkX: G, // left edge of "flowmaid"
  centerY: HEADER_H / 2, // 48
  wordmarkSize: 44, // Archivo 900
  eyebrow: 'PURE RUST · ZERO DEPS',
  // INK|PAPER toggle: right-aligned on the gutter, segments 76px (INK) + 100px (PAPER), 1px
  // border → outer box x 846..1024. The mode-switch circle grows from the CENTRE OF THE
  // BUTTON BEING SWITCHED TO (INK centre when going to Ink, PAPER centre going to Paper).
  toggle: { left: 846, inkCenter: { x: 885, y: 48 }, paperCenter: { x: 973, y: 48 } },
} as const;

/** circle radius that covers the whole frame from (x, y), ×1.1 (theme-switch card rule). */
export const coverRadius = (x: number, y: number) =>
  1.1 * Math.max(Math.hypot(x, y), Math.hypot(1080 - x, y), Math.hypot(x, 1350 - y), Math.hypot(1080 - x, 1350 - y));

export const FOOTER_LEFT = ['pure rust', 'zero deps', 'webassembly'];
export const FOOTER_RIGHT = 'gpl-3.0-or-later';

// ───────────────────────── motion personality ─────────────────────────
// Brand → motion preset: engineering dev-tool, "professional trust" family
// (pipeline.md table): main duration ~21f, entrance bezier(0,0,0.2,1), overshoot 1.0
// (no bounce on fades/pushes), squash 0. Exception (card rule wins): real LANDINGS
// (outro group-photo tiles) keep y1 > 1. All values below are frame-pure.

const clamp01 = (x: number) => (x < 0 ? 0 : x > 1 ? 1 : x);

export const MOTION = {
  mainDur: 21, // default entrance length (frames)
  wipeFrames: 16, // Paper→Paper / Ink→Ink seam (accent line wipe, fx.html 0.55s)
  knobFrames: 9, // INK|PAPER knob slide (fx.html 0.3s)
  circleFrames: 22, // mode-switch circle (fx.html 0.75s)
  holdMin: 30, // R1 brand / key-info hold
  restMin: 15, // R2 rest after batch motion
} as const;

// Easings from the approved fx.html (keep these names across all shots).
export const ease = {
  /** fx `quiet`: cubic ease-out. Secondary fades / rises. */
  quiet: (x: number) => 1 - Math.pow(1 - clamp01(x), 3),
  /** fx `expo`: exponential ease-out. Lines drawing, masks, knobs. */
  expo: (x: number) => {
    const t = clamp01(x);
    return t === 1 ? 1 : 1 - Math.pow(2, -10 * t);
  },
  /** fx `inout`: cubic in-out. Wipes, mode-switch circle, camera moves. */
  inout: (x: number) => {
    const t = clamp01(x);
    return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
  },
  /** professional-trust entrance, cubic-bezier(0,0,0.2,1). */
  enter: (x: number) => bezier(0, 0, 0.2, 1)(x),
};

/** progress of `frame` through [start, start+dur], clamped 0..1 (fx `prog`). */
export const prog = (frame: number, start: number, dur: number) => clamp01((frame - start) / Math.max(1e-6, dur));
export const mix = (a: number, b: number, t: number) => a + (b - a) * t;

// cubic-bezier → (t)=>y (same as CSS). Copied from demos/_fixtures/Polish.tsx.
export function bezier(x1: number, y1: number, x2: number, y2: number) {
  const ax = 1 - 3 * x2 + 3 * x1, bx = 3 * x2 - 6 * x1, cx = 3 * x1;
  const ay = 1 - 3 * y2 + 3 * y1, by = 3 * y2 - 6 * y1, cy = 3 * y1;
  const sx = (u: number) => ((ax * u + bx) * u + cx) * u;
  const sy = (u: number) => ((ay * u + by) * u + cy) * u;
  const dx = (u: number) => (3 * ax * u + 2 * bx) * u + cx;
  const solve = (x: number) => {
    let u = x;
    for (let i = 0; i < 8; i++) {
      const err = sx(u) - x;
      if (Math.abs(err) < 1e-7) return u;
      const d = dx(u);
      if (Math.abs(d) < 1e-6) break;
      u -= err / d;
    }
    let lo = 0, hi = 1;
    u = x;
    for (let i = 0; i < 40; i++) {
      const v = sx(u);
      if (Math.abs(v - x) < 1e-7) break;
      if (v < x) lo = u; else hi = u;
      u = (lo + hi) / 2;
    }
    return u;
  };
  return (t: number) => (t <= 0 ? 0 : t >= 1 ? 1 : sy(solve(t)));
}

// ───────────────────────── shared text styles ─────────────────────────

/** mono uppercase label (scene label rows, pane headers). */
export const labelStyle = (p: Palette, size = 22): CSSProperties => ({
  fontFamily: FONT.mono, fontWeight: 500, fontSize: size, lineHeight: 1,
  letterSpacing: TYPE.labelTracking, textTransform: 'uppercase', color: p.dim,
});

/** accent tag chip ("01"), as in the style frames. */
export const tagStyle = (p: Palette, size = 20): CSSProperties => ({
  display: 'inline-block', padding: '7px 11px', background: p.accentSoft, color: p.accent,
  fontFamily: FONT.mono, fontWeight: 500, fontSize: size, lineHeight: 1, letterSpacing: TYPE.labelTracking,
  textTransform: 'uppercase',
});

/** Archivo 900 display text. */
export const displayStyle = (p: Palette, size: number): CSSProperties => ({
  fontFamily: FONT.display, fontWeight: 900, fontSize: size, lineHeight: 0.95,
  letterSpacing: TYPE.displayTracking, color: p.ink,
});
