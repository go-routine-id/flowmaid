// Shot 3 `speed`: the LOG speed tape inside the "RENDER SPEED" pane body (body-local coords,
// 422 × 771, pointer at the body's vertical centre). "World moves, pointer stays": the whole
// scale slides behind a fixed accent pointer; larger values sit above.
//
// Layers (bottom → top): ticks + MERMAID.JS limit line + FLOWMAID marker (speed-blurred) →
// fixed pointer line (never blurred) → labels on panel chips (speed-blurred; a chip knocks the
// pointer line out behind a label that sits on it, e.g. "1×" while parked).
// All tape y positions are rounded to whole pixels so the parked and locked frames are crisp.
import React from 'react';
import { FONT, TYPE, ease, prog, type Palette } from '../../tokens';
import { EASE, SpeedBlur, ramp } from '../../lib/Polish';
import { SPEED_BEATS as B } from './beats';
import { PX_PER_DECADE, V_LOCK } from './speedModel';

export const TAPE = {
  W: 422, // pane body width (pane w 424 minus the two 1px rules)
  H: 771, // pane body height (820 − 48 header − 1)
  CY: 385, // pointer line centre (body-local); canvas y = 104 + 48 + 385 = 537
  TICK_R: 394, // ticks end here (right), extend leftwards
  MAJOR: 120,
  MINOR: 60,
  LABEL_R: 256, // right edge of the tick-label column
  LEFT: 22, // left labels (MERMAID.JS / FLOWMAID)
} as const;

const MAJORS = new Set([1, 10, 100]);
const SMALL = new Set([2, 5, 20, 50, 200]);
const BLUR = { amount: 0.2, max: 14 }; // peak tape speed ≈ 83 px/f → capped at 14 px

/** the scale: m × 10^k (m = 1..9) from the 1× MERMAID.JS baseline up to 9000×. Nothing is drawn
 *  below 1×: the scale starts at the mermaid.js baseline (and a bunched sub-1× decade read as noise). */
const TICKS: { u: number; lg: number }[] = (() => {
  const out: { u: number; lg: number }[] = [];
  for (let k = 0; k <= 3; k++) {
    for (let m = 1; m <= 9; m++) out.push({ u: m * 10 ** k, lg: Math.log10(m) + k });
  }
  return out;
})();

export const Tape: React.FC<{ p: Palette; frame: number; lg: number; vy: number }> = ({ p, frame, lg, vy }) => {
  const { W, H, CY, TICK_R, MAJOR, MINOR, LABEL_R, LEFT } = TAPE;
  const yOfLg = (l: number) => Math.round(CY + (lg - l) * PX_PER_DECADE);
  const inView = (y: number, pad = 60) => y > -pad && y < H + pad;

  const tapeIn = ease.quiet(prog(frame, B.tapeIn, 10));
  const pointerDraw = ease.expo(prog(frame, B.pointerIn, 12));

  // MERMAID.JS limit line (at 1×): breaks at B.breakAt — halves slide 8 px apart and fade.
  const brk = ramp(frame, B.breakAt, B.breakDur, EASE.out);
  const yM = yOfLg(0);

  // FLOWMAID marker (at 113×): comes up to strength as it nears the pointer, full at the lock.
  const yF = yOfLg(Math.log10(V_LOCK));
  const near = Math.max(0, 1 - Math.abs(yF - CY) / 260);
  const locked = ease.quiet(prog(frame, B.lockAt, B.lockFx));
  const markerO = Math.min(1, 0.55 * ease.quiet(near) + 0.45 * locked);

  const ticks: React.ReactNode[] = [];
  const labels: React.ReactNode[] = [];
  for (const t of TICKS) {
    const y = yOfLg(t.lg);
    if (!inView(y)) continue;
    const major = MAJORS.has(t.u);
    const len = major ? MAJOR : MINOR;
    const th = major ? 2 : 1;
    ticks.push(
      <div
        key={`t${t.u}`}
        style={{ position: 'absolute', left: TICK_R - len, width: len, top: y - (major ? 1 : 0), height: th, background: major ? p.ink : p.dim }}
      />,
    );
    if (major || SMALL.has(t.u)) {
      const fs = major ? 28 : 20;
      const ch = fs + 2; // chip height (knocks the pointer out only behind a label sitting ON it)
      labels.push(
        <div
          key={`l${t.u}`}
          style={{
            position: 'absolute', right: W - LABEL_R, top: y - ch / 2, height: ch, lineHeight: `${ch}px`, padding: '0 6px',
            background: p.panel, fontFamily: FONT.mono, fontWeight: 500, fontSize: fs, color: major ? p.ink : p.muted,
            fontVariantNumeric: 'tabular-nums', whiteSpace: 'nowrap',
          }}
        >
          {t.u}×
        </div>,
      );
    }
  }

  const dash = '7 5';
  const half = W / 2;

  return (
    <div style={{ position: 'absolute', inset: 0, opacity: tapeIn }}>
      {/* ── tape: lines (blurred while it runs) ── */}
      <SpeedBlur vx={0} vy={vy} amount={BLUR.amount} max={BLUR.max}>
        {ticks}
        {brk < 1 && inView(yM) ? (
          <svg width={W} height={H} style={{ position: 'absolute', left: 0, top: 0, overflow: 'visible', opacity: 1 - brk }}>
            <line x1={0} x2={half} y1={yM + 0.5} y2={yM + 0.5} stroke={p.dim} strokeWidth={1} strokeDasharray={dash} transform={`translate(${(-8 * brk).toFixed(3)} 0)`} />
            <line x1={half} x2={W} y1={yM + 0.5} y2={yM + 0.5} stroke={p.dim} strokeWidth={1} strokeDasharray={dash} strokeDashoffset={half} transform={`translate(${(8 * brk).toFixed(3)} 0)`} />
          </svg>
        ) : null}
        {markerO > 0.001 && inView(yF) ? (
          <svg width={W} height={H} style={{ position: 'absolute', left: 0, top: 0, overflow: 'visible', opacity: markerO }}>
            <rect x={TICK_R - MAJOR} y={yF - 1} width={MAJOR} height={2} fill={p.accent} />
            <polygon points={`${TICK_R + 6},${yF} ${TICK_R + 20},${yF - 8} ${TICK_R + 20},${yF + 8}`} fill={p.accent} />
          </svg>
        ) : null}
      </SpeedBlur>

      {/* ── fixed pointer line (never moves, never blurs) ── */}
      <div
        style={{
          position: 'absolute', left: 0, width: W, top: CY - 1, height: 2, background: p.accent,
          transformOrigin: 'left', transform: `scaleX(${pointerDraw.toFixed(4)})`,
        }}
      />

      {/* ── tape: labels (blurred with the tape, above the pointer) ── */}
      <SpeedBlur vx={0} vy={vy} amount={BLUR.amount} max={BLUR.max}>
        {labels}
        {inView(yM) ? (
          <div
            style={{
              position: 'absolute', left: LEFT, top: yM - 8 - 18, height: 18, lineHeight: '18px', fontFamily: FONT.mono, fontWeight: 500,
              fontSize: 18, letterSpacing: TYPE.labelTracking, textTransform: 'uppercase', color: p.dim, whiteSpace: 'nowrap',
            }}
          >
            mermaid.js
          </div>
        ) : null}
        {markerO > 0.001 && inView(yF) ? (
          <div
            style={{
              position: 'absolute', left: LEFT, top: yF - 9 - 20, height: 20, lineHeight: '20px', fontFamily: FONT.mono, fontWeight: 600,
              fontSize: 20, letterSpacing: TYPE.labelTracking, textTransform: 'uppercase', color: p.accent, whiteSpace: 'nowrap', opacity: markerO,
            }}
          >
            flowmaid
          </div>
        ) : null}
      </SpeedBlur>
    </div>
  );
};
