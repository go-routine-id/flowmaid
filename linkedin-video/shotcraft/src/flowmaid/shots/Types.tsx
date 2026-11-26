// TypesShot — shot 4 `types` (abs 341–451, 110 f, Paper). Package B (fixer round 1: re-timed).
// Card: wall-reveal-moves · style wireframe-draw-on (ref demos/ui-entrance/wall-reveal-moves/
// WireframeDrawOn.tsx), re-skinned to the playground: a 3 × 4 wall of the 12 real flowmaid renders.
//   6–42   blueprint: grid rules draw on in staggered groups, then a WIRE version of every real SVG
//          (same markup, fills off, muted 1 px strokes, text hidden) draws itself; labels muted
//   42–48  breath
//   48–78  one value s (card bezier .55,0,.25,1) drives both a 2 px accent line and the clip of
//          the materialised layer (real SVGs, ink labels, accent indices), TOP → BOTTOM so it
//          follows the VO's reading order "from flowcharts to sankey"; the counter steps per row
//          and lands on 12/12 (accent) at COUNTER_DONE (78)
//   78–110 the materialised wall holds 32 f (scan line gone at 80, then 30 f fully still; the last frame
//          is held under the next wipe). All beats live in shots/b/beats.ts (TYPES_BEATS).
// Cell 01 shows svg/hero.svg (the flowchart of shot 2) instead of svg/advanced.svg: advanced is
// 596 × 1236 and rendered as a pale 67 px wide strip in the 297 × 140 cell body (review nit).
// Deviations from the card (DESIGN.md 10): top→bottom instead of left→right; no glow, no warm
// afterglow, no vignette/grain; Paper tokens.
//
// Canvas-local coordinates (0,0 = frame (0,96)); useCurrentFrame() is shot-local.
import React, { useMemo } from 'react';
import { AbsoluteFill, Img, staticFile, useCurrentFrame } from 'remotion';
import { FONT, PAPER, TYPE, ease, prog } from '../tokens';
import { SceneLabel } from '../lib/Pane';
import { COUNTER_DONE, TYPES_BEATS as B, blueprintDraw, counterAt, scanAt } from './b/beats';
import { Odometer } from './b/Odometer';
import { useSvgTexts, viewBoxOf, wireMarkup } from './b/wire';

const p = PAPER;

// ── wall geometry (pixel rows/columns that carry the 1 px rules) ──
// Grid x 56..1024, y 100..920: 3 columns (~322.7) × 4 rows (205). DESIGN.md asks for rows of 210
// (to 940); at 940 the bottom rule sat 6 px above the 2-line VO4 caption box (canvas ~945) and the
// two framed boxes read as one double rule, so the rows are 5 px shorter (deviation, reported).
const COLS = [56, 379, 701, 1023];
const ROWS = [100, 305, 510, 715, 920];
const HEAD = 40; // header strip; its rule sits on row y0 + 40
const PAD = 12;
const GRID = { x: 56, y: 100, w: 968, h: 821 } as const; // scan/clip box: rows 100..920 inclusive

const TYPES = [
  { label: 'Flowchart', file: 'svg/hero.svg' },
  { label: 'Sequence', file: 'svg/sequence.svg' },
  { label: 'Class', file: 'svg/class.svg' },
  { label: 'State', file: 'svg/state.svg' },
  { label: 'ER', file: 'svg/er.svg' },
  { label: 'Pie', file: 'svg/pie.svg' },
  { label: 'Journey', file: 'svg/journey.svg' },
  { label: 'Git graph', file: 'svg/gitgraph.svg' },
  { label: 'Mindmap', file: 'svg/mindmap.svg' },
  { label: 'Timeline', file: 'svg/timeline.svg' },
  { label: 'Sankey', file: 'svg/sankey.svg' },
  { label: 'Architecture', file: 'svg/architecture.svg' },
] as const;
const FILES = TYPES.map((t) => t.file);

type Cell = { i: number; x0: number; x1: number; y0: number; y1: number; bx: number; by: number; bw: number; bh: number };
const CELLS: Cell[] = TYPES.map((_, i) => {
  const c = i % 3;
  const r = Math.floor(i / 3);
  const x0 = COLS[c], x1 = COLS[c + 1], y0 = ROWS[r], y1 = ROWS[r + 1];
  const bx = x0 + 1 + PAD, by = y0 + HEAD + 1 + PAD;
  return { i, x0, x1, y0, y1, bx, by, bw: x1 - 1 - PAD - bx, bh: y1 - PAD - by };
});

// blueprint timing (DESIGN.md 8.4; constants in TYPES_BEATS)
const draw = blueprintDraw; // card's line-draw easing bezier(0.4, 0, 0.3, 1)
const T_COL = (c: number) => B.colAt + B.colStep * c;
const T_ROW = (r: number) => B.rowAt + B.rowStep * r;
const T_HEAD = (i: number) => B.headAt + B.headStep * i;
const T_WIRE = (i: number) => B.wireAt + B.wireStep * i;

const labelText: React.CSSProperties = {
  position: 'absolute', height: HEAD - 1, lineHeight: `${HEAD - 1}px`, fontFamily: FONT.mono, fontWeight: 500, fontSize: 19,
  letterSpacing: TYPE.labelTracking, textTransform: 'uppercase', whiteSpace: 'nowrap',
};

/** header strip text of one cell (label left, index right) in the given colours. */
const HeaderText: React.FC<{ cell: Cell; label: string; labelColor: string; indexColor: string; opacity?: number }> = ({
  cell, label, labelColor, indexColor, opacity = 1,
}) => (
  <>
    <div style={{ ...labelText, left: cell.x0 + 15, top: cell.y0 + 1, color: labelColor, opacity }}>{label}</div>
    <div style={{ ...labelText, right: 1080 - (cell.x1 - 14), top: cell.y0 + 1, color: indexColor, opacity }}>
      {String(cell.i + 1).padStart(2, '0')}
    </div>
  </>
);

/** a 1 px rule that draws on (pathLength 1). */
const Rule: React.FC<{ d: string; t: number }> = ({ d, t }) =>
  t <= 0 ? null : (
    <path d={d} fill="none" stroke={p.line2} strokeWidth={1} pathLength={1} strokeDasharray="1.002 1.002" strokeDashoffset={(1.002 * (1 - t)).toFixed(5)} />
  );

export const TypesShot: React.FC = () => {
  const frame = useCurrentFrame();
  const texts = useSvgTexts(FILES);

  // per cell: contained box (shared by the wire and the real layer) + wire markup
  const placed = useMemo(() => {
    if (!texts) return null;
    return CELLS.map((cell, i) => {
      const vb = viewBoxOf(texts[i]);
      const s = Math.min(cell.bw / vb.w, cell.bh / vb.h);
      const w = vb.w * s, h = vb.h * s;
      const box = { left: cell.bx + (cell.bw - w) / 2, top: cell.by + (cell.bh - h) / 2, width: w, height: h };
      return { box, wire: wireMarkup(texts[i], p.muted, 1 / s) };
    });
  }, [texts]);

  const s = scanAt(frame);
  const lineO = Math.min(ease.quiet(prog(frame, B.lineIn, 4)), 1 - ease.quiet(prog(frame, B.lineOut, 4)));
  const clipBottom = 1200 - (GRID.y + s * GRID.h);

  // counter: 00 → 03 → 06 → 09 → 12 as the scan finishes each row
  const count = counterAt(frame);
  const done = frame >= COUNTER_DONE; // = first frame the roll has landed on 12
  const counterColor = done ? p.accent : p.dim;
  const MONO_ADV = 0.6 * 22;
  const counterStyle: React.CSSProperties = { fontFamily: FONT.mono, fontWeight: 500, fontSize: 22, color: counterColor };

  // rules
  const outer = draw(frame, B.frameAt, B.frameDur);
  const L = COLS[0] + 0.5, R = COLS[3] + 0.5, T = ROWS[0] + 0.5, Bt = ROWS[4] + 0.5;

  return (
    <AbsoluteFill>
      <SceneLabel
        p={p}
        frame={frame}
        n="03"
        label="DIAGRAM TYPES"
        right={
          <span style={{ display: 'inline-flex', alignItems: 'flex-start', height: 22, lineHeight: '22px', letterSpacing: TYPE.labelTracking }}>
            <Odometer p0={count} columns={2} pad pitch={MONO_ADV + 0.07 * 22} glyphW={MONO_ADV + 2} h={22} textStyle={counterStyle} />
            <span style={{ ...counterStyle, marginLeft: 0.07 * 22 - 2, lineHeight: '22px' }}>/12</span>
          </span>
        }
      />

      {/* ── blueprint layer: wire SVGs + muted labels ── */}
      {placed
        ? CELLS.map((cell, i) => {
            const t = draw(frame, T_WIRE(i), B.wireDur);
            return (
              <div
                key={`w${i}`}
                style={{ position: 'absolute', ...placed[i].box, opacity: t > 0 ? 1 : 0, ['--t' as string]: t.toFixed(5) } as React.CSSProperties}
                dangerouslySetInnerHTML={{ __html: placed[i].wire }}
              />
            );
          })
        : null}
      {CELLS.map((cell, i) => (
        <HeaderText key={`bl${i}`} cell={cell} label={TYPES[i].label} labelColor={p.muted} indexColor={p.muted} opacity={ease.quiet(prog(frame, T_HEAD(i) + 2, 8))} />
      ))}

      {/* ── materialised layer: revealed top → bottom by the scan ── */}
      {s > 0 ? (
        <div style={{ position: 'absolute', inset: 0, clipPath: `inset(${GRID.y}px ${1080 - (GRID.x + GRID.w)}px ${clipBottom.toFixed(3)}px ${GRID.x}px)` }}>
          <div style={{ position: 'absolute', left: GRID.x, top: GRID.y, width: GRID.w, height: GRID.h, background: p.canvas }} />
          {CELLS.map((cell, i) => (
            <React.Fragment key={`m${i}`}>
              <div style={{ position: 'absolute', left: cell.x0, top: cell.y0, width: cell.x1 - cell.x0, height: HEAD, background: p.bg }} />
              {placed ? <Img src={staticFile(TYPES[i].file)} style={{ position: 'absolute', ...placed[i].box }} /> : null}
              <HeaderText cell={cell} label={TYPES[i].label} labelColor={p.ink} indexColor={p.accent} />
            </React.Fragment>
          ))}
        </div>
      ) : null}

      {/* ── rules (shared by both layers, always on top) ── */}
      <svg width={1080} height={1200} style={{ position: 'absolute', left: 0, top: 0 }}>
        <Rule d={`M${L},${T} H${R} V${Bt} H${L} Z`} t={outer} />
        {[1, 2].map((c) => (
          <Rule key={`c${c}`} d={`M${COLS[c] + 0.5},${T} V${Bt}`} t={draw(frame, T_COL(c), B.gridDur)} />
        ))}
        {[1, 2, 3].map((r) => (
          <Rule key={`r${r}`} d={`M${L},${ROWS[r] + 0.5} H${R}`} t={draw(frame, T_ROW(r), B.gridDur)} />
        ))}
        {CELLS.map((cell, i) => (
          <Rule key={`h${i}`} d={`M${cell.x0 + 1},${cell.y0 + HEAD + 0.5} H${cell.x1}`} t={draw(frame, T_HEAD(i), B.headDur)} />
        ))}
      </svg>

      {/* ── scan line: rides the clip edge (same s) ── */}
      {lineO > 0 ? (
        <div style={{ position: 'absolute', left: GRID.x, width: GRID.w, top: GRID.y + s * GRID.h - 1, height: 2, background: p.accent, opacity: lineO }} />
      ) : null}
    </AbsoluteFill>
  );
};
