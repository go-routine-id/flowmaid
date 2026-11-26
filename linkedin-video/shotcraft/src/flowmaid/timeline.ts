// flowmaid promo — THE timeline. One source of truth (skill rule): shot durations are
// written once here; every `from`, the caption table, the transition points and the
// cue sheet are DERIVED from SHOTS and exported. Never write a bare absolute frame
// number anywhere else — use SHOTS.<id>.from + localFrame.
//
// 1080×1350 @ 30 fps. Total 1080 f = 36.00 s.
import { HEADER, MOTION, coverRadius, ease, type Mode } from './tokens';

export const FPS = 30;

// ───────────────────────── shots ─────────────────────────
// Edit ONLY `duration` (and order) here; `from` is computed.
const SHOT_DEFS = [
  { id: 'open', duration: 90, mode: 'paper', title: 'Brand open', card: 'brand-ink-open', style: 'brand-ink-open' },
  { id: 'render', duration: 138, mode: 'paper', title: 'Text in, SVG out', card: 'canvas-materialize-moves', style: 'diagram-cascade' },
  { id: 'speed', duration: 113, mode: 'paper', title: '~113x faster', card: 'gauge-readout-moves', style: 'tape-scroll-fixed-pointer' },
  { id: 'types', duration: 110, mode: 'paper', title: '12 diagram types', card: 'wall-reveal-moves', style: 'wireframe-draw-on' },
  { id: 'browser', duration: 150, mode: 'paper', title: 'flowmaid-web drag', card: 'crash-zoom-punch', style: 'crash-zoom-punch' },
  { id: 'flowcli', duration: 156, mode: 'ink', title: 'flowcli --watch', card: 'speed-ramp-freeze', style: 'freeze-annotate' },
  { id: 'dbx', duration: 150, mode: 'ink', title: 'dbx ERD', card: 'beat-cut-moves', style: 'paparazzi-flash' },
  { id: 'outro', duration: 173, mode: 'paper', title: 'Open source / CTA', card: 'outro-group-photo-launch', style: 'outro-group-photo-launch' },
] as const;

export type ShotId = (typeof SHOT_DEFS)[number]['id'];
export type Shot = {
  id: ShotId; index: number; from: number; duration: number; end: number; mode: Mode;
  title: string; card: string; style: string;
};

export const SHOT_ORDER: ShotId[] = SHOT_DEFS.map((s) => s.id);

export const SHOTS: Record<ShotId, Shot> = (() => {
  let from = 0;
  const out = {} as Record<ShotId, Shot>;
  SHOT_DEFS.forEach((s, index) => {
    out[s.id] = { ...s, mode: s.mode as Mode, index, from, end: from + s.duration };
    from += s.duration;
  });
  return out;
})();

export const TOTAL = SHOT_DEFS.reduce((n, s) => n + s.duration, 0); // 1080

export const shotAt = (frame: number): Shot => {
  for (const id of SHOT_ORDER) {
    const s = SHOTS[id];
    if (frame >= s.from && frame < s.end) return s;
  }
  return SHOTS[SHOT_ORDER[SHOT_ORDER.length - 1]];
};

// ───────────────────────── transitions (derived) ─────────────────────────
// Every seam is at the incoming shot's `from` (the cut point).
//  - 'handoff' : open → render. The S01 wordmark docks into the header (HEADER_HANDOFF);
//                no wipe.
//  - 'wipe'    : same mode. Incoming is revealed left→right by clip-path, a 2px accent
//                line rides the edge (fx.html). Runs over the incoming's first 16 f; the
//                outgoing is shown FROZEN on its last frame underneath.
//  - 'mode'    : mode change (approved mode switch). The outgoing header knob slides to the
//                new mode in the outgoing's last 9 f, then the incoming layer (with its own
//                chrome) grows as a circle out of the toggle button over the incoming's
//                first 22 f; outgoing frozen underneath.
// Contract for shot implementers: your first 16 f (wipe) / 22 f (mode) are being
// revealed — start entrances ~local 4–10; your last 9 f before a 'mode' seam carry the
// knob slide — keep them calm. Your last frame is shown frozen during the next seam.

export type TransitionKind = 'handoff' | 'wipe' | 'mode';
export type Transition = {
  kind: TransitionKind; from: ShotId; to: ShotId; at: number; // cut frame (= SHOTS[to].from)
  frames: number; // reveal length after the cut
  knobFrom?: number; // 'mode': knob slide start (absolute)
  toMode: Mode;
};

export const TRANSITIONS: Transition[] = SHOT_ORDER.slice(1).map((id, i) => {
  const prev = SHOTS[SHOT_ORDER[i]];
  const next = SHOTS[id];
  if (prev.id === 'open') return { kind: 'handoff', from: prev.id, to: id, at: next.from, frames: 0, toMode: next.mode };
  if (prev.mode !== next.mode) {
    return {
      kind: 'mode', from: prev.id, to: id, at: next.from, frames: MOTION.circleFrames,
      knobFrom: next.from - MOTION.knobFrames, toMode: next.mode,
    };
  }
  return { kind: 'wipe', from: prev.id, to: id, at: next.from, frames: MOTION.wipeFrames, toMode: next.mode };
});

export const transitionInto = (id: ShotId): Transition | undefined => TRANSITIONS.find((t) => t.to === id);
export const transitionOutOf = (id: ShotId): Transition | undefined => TRANSITIONS.find((t) => t.from === id);

// Header handoff in the opening: the big S01 wordmark flies into the header slot over
// [from, to); the chrome header (rule, eyebrow, toggle, footer) draws in over the same
// window; the chrome's own wordmark becomes visible at `to` (S01 overlay hides then).
export const HEADER_HANDOFF = { from: SHOTS.open.from + 74, to: SHOTS.open.from + 88 } as const;

/** 0..1 progress of the header strip drawing in (1 everywhere after the opening). */
export const headerProgressAt = (frame: number) =>
  Math.max(0, Math.min(1, (frame - HEADER_HANDOFF.from) / (HEADER_HANDOFF.to - HEADER_HANDOFF.from)));
export const chromeWordmarkVisibleAt = (frame: number) => frame >= HEADER_HANDOFF.to;

/** knob position for a layer of mode `mode` at `frame`: 0 = INK selected, 1 = PAPER selected. */
export const knobAt = (frame: number, mode: Mode): number => {
  const base = mode === 'paper' ? 1 : 0;
  for (const t of TRANSITIONS) {
    if (t.kind !== 'mode' || t.knobFrom === undefined) continue;
    // the OUTGOING layer's knob slides toward the new mode during [knobFrom, at]
    if (SHOTS[t.from].mode === mode && frame >= t.knobFrom && frame < t.at + t.frames) {
      const p = ease.inout((frame - t.knobFrom) / MOTION.knobFrames);
      const target = t.toMode === 'paper' ? 1 : 0;
      return base + (target - base) * p;
    }
  }
  return base;
};

// ───────────────────────── voice-over + captions ─────────────────────────
// The owner records the VO later (ElevenLabs). Captions = the VO lines, burned in.
// To re-time a caption: change `lead` (frames after its shot's from) and `tail` (frames
// before its shot's end) — or split it with `parts`. Nothing else needs to move.

export type VoLine = {
  n: number; shot: ShotId; text: string; estSeconds: number; lead: number; tail: number;
  parts?: { text: string; until: number }[]; // optional split: each part shows until local frame `until`
};

export const VO_LINES: VoLine[] = [
  { n: 1, shot: 'open', text: 'flowmaid renders Mermaid diagrams in pure Rust.', estSeconds: 2.7, lead: 4, tail: 2 },
  { n: 2, shot: 'render', text: 'Text in, clean SVG out.', estSeconds: 2.2, lead: 10, tail: 4 },
  { n: 3, shot: 'speed', text: 'Over a hundred times faster than mermaid.js.', estSeconds: 2.9, lead: 10, tail: 4 },
  { n: 4, shot: 'types', text: 'Twelve diagram types, from flowcharts to sankey.', estSeconds: 2.9, lead: 12, tail: 4 },
  { n: 5, shot: 'browser', text: 'In the browser, drag a node, and its edges follow.', estSeconds: 4.3, lead: 12, tail: 6 },
  { n: 6, shot: 'flowcli', text: 'flowcli draws them in your terminal, and redraws when you save.', estSeconds: 4.5, lead: 22, tail: 4 },
  { n: 7, shot: 'dbx', text: 'In dbx, flowmaid lays out your database schema as an ERD.', estSeconds: 4.5, lead: 12, tail: 8 },
  {
    n: 8, shot: 'outro', text: 'flowmaid is open source. Try it in your browser, or cargo add flowmaid.', estSeconds: 5.5,
    lead: 22, tail: 2,
    parts: [
      { text: 'flowmaid is open source.', until: 78 },
      { text: 'Try it in your browser, or cargo add flowmaid.', until: 171 }, // last part ends at end - tail
    ],
  },
];

export type Caption = { id: string; vo: number; shot: ShotId; mode: Mode; text: string; from: number; to: number };

export const CAPTIONS: Caption[] = VO_LINES.flatMap((v) => {
  const s = SHOTS[v.shot];
  const start = s.from + v.lead;
  const end = s.end - v.tail;
  if (!v.parts) return [{ id: `vo${v.n}`, vo: v.n, shot: v.shot, mode: s.mode, text: v.text, from: start, to: end }];
  let cursor = start;
  return v.parts.map((p, i) => {
    const to = i === v.parts!.length - 1 ? end : Math.min(end, s.from + p.until);
    const c = { id: `vo${v.n}${String.fromCharCode(97 + i)}`, vo: v.n, shot: v.shot, mode: s.mode, text: p.text, from: cursor, to };
    cursor = to;
    return c;
  });
});

// ───────────────────────── cue sheet ─────────────────────────
// Visual hits for the owner's sound design (the master is SILENT). Each shot package
// keeps its own local cue list in shots/<Name>.cues.ts; cues.ts maps them through
// SHOTS. Transition cues are generated here.

export type LocalCue = { at: number; action: string; sound: string };
export type Cue = { frame: number; time: string; shot: ShotId | 'seam'; action: string; sound: string };

export const fmtTime = (frame: number) => {
  const t = frame / FPS;
  const m = Math.floor(t / 60);
  const s = t - m * 60;
  return `${String(m).padStart(2, '0')}:${s.toFixed(2).padStart(5, '0')}`;
};

/** absolute cue from a shot-local frame. */
export const cue = (shot: ShotId, local: number, action: string, sound: string): Cue => {
  const frame = SHOTS[shot].from + local;
  return { frame, time: fmtTime(frame), shot, action, sound };
};

export const TRANSITION_CUES: Cue[] = TRANSITIONS.flatMap((t): Cue[] => {
  if (t.kind === 'wipe') {
    return [{ frame: t.at, time: fmtTime(t.at), shot: 'seam', action: `accent-line wipe ${t.from} -> ${t.to}`, sound: 'soft swoosh, short (~0.5 s)' }];
  }
  if (t.kind === 'mode' && t.knobFrom !== undefined) {
    // Derived from the same reveal Main.tsx draws (radius = ease.inout(k / frames) x coverRadius):
    // the cut frame itself shows nothing, so the whoosh cue sits on the first frame the circle is
    // visible (radius >= 4 px inside the toggle button) and a second cue marks the fastest growth.
    const c = t.toMode === 'ink' ? HEADER.toggle.inkCenter : HEADER.toggle.paperCenter;
    const R = coverRadius(c.x, c.y);
    let first = 1;
    while (first < t.frames && ease.inout(first / t.frames) * R < 4) first++;
    const peak = t.at + Math.round(t.frames / 2); // cubic in-out: fastest at mid-reveal
    const end = t.at + t.frames;
    return [
      { frame: t.knobFrom, time: fmtTime(t.knobFrom), shot: 'seam', action: `mode toggle knob slides to ${t.toMode.toUpperCase()}`, sound: 'mechanical toggle click (real switch, not a UI bleep)' },
      { frame: t.at + first, time: fmtTime(t.at + first), shot: 'seam', action: `${t.toMode} layer circle appears in the ${t.toMode.toUpperCase()} button (${t.from} -> ${t.to}; full frame covered at ${end})`, sound: `airy whoosh ~0.7 s starts here, peaks ${peak - t.at - first} f later` },
      { frame: peak, time: fmtTime(peak), shot: 'seam', action: `${t.toMode} circle grows fastest (mid-reveal)`, sound: 'whoosh peak' },
    ];
  }
  return [];
});
