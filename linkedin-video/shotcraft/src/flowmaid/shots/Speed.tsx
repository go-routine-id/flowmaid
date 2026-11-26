// SpeedShot — shot 3 `speed` (abs 228–341, 113 f, Paper). Package B (fixer round 1: re-timed).
// Card: gauge-readout-moves · style tape-scroll-fixed-pointer
// (ref demos/data/gauge-readout-moves/TapeScrollFixedPointer.tsx), re-skinned to the playground:
// a LOG speed tape (300 px / decade) scrolls behind a fixed accent pointer — parked on the
// MERMAID.JS line at 1×, the line breaks, a short pull-back, a sprint with vertical speed blur,
// a spring brake with one swing, and the readout locks on "~113×". Claim + mandatory footnote
// exactly as in BRIEF.md; no other numbers besides the scale labels DESIGN.md lists.
// Beats (shots/b/beats.ts): parked 0-28, line breaks 28, anticipation 32, sprint 37-59, brake 59-75,
// LOCK 75 (claim rises 71-81, "~" 75-81), then every pixel still from 81 to the cut at 113 (32 f, card + R1). The footnote that
// qualifies the claim comes in with the readout (16-28) and stays for the whole shot (~3 s), at the
// 32 px aux size in --dim (Q11), instead of a 0.5 s muted 28 px fade after the lock.
//
// Canvas-local coordinates (0,0 = frame (0,96)); useCurrentFrame() is shot-local. Main reveals
// the first 16 f with the accent-line wipe and freezes the last frame under the next wipe.
import React from 'react';
import { AbsoluteFill, useCurrentFrame } from 'remotion';
import { FONT, PAPER, TYPE, ease, labelStyle, prog } from '../tokens';
import { Pane, PANE_HEADER_H, SceneLabel } from '../lib/Pane';
import { velocity } from '../lib/Polish';
import { SPEED_BEATS as B } from './b/beats';
import { Odometer, detent } from './b/Odometer';
import { PX_PER_DECADE, logValueAt, valueAt } from './b/speedModel';
import { TAPE, Tape } from './b/Tape';

const p = PAPER;

// tape pane (DESIGN.md 8.3)
const PANE = { x: 600, y: 104, w: 424, h: 820 } as const;
const CY = PANE.y + PANE_HEADER_H + TAPE.CY; // pointer line, canvas y (537)

// readout: Archivo 900, sized so the locked "~113×" fills the x 56..560 column.
// Archivo 900 advances: digits 0.667 em, "~" and "×" 0.660 em; tracking −0.045 em.
const FS = 160;
const PITCH = (0.667 - 0.045) * FS; // digit column pitch
const GLYPH_W = 0.667 * FS;
const SIGN_PITCH = (0.66 - 0.045) * FS; // "~" / "×"
const RX = 56; // "~" starts on the gutter once locked
const ODO_X = RX + SIGN_PITCH; // three digit columns
const TIMES_X = ODO_X + 3 * PITCH;
const WIN_TOP = CY - FS / 2; // digit window (one digit pitch tall), centred on the pointer
const READ_W = 504; // readout column x 56..560

const digitStyle: React.CSSProperties = {
  fontFamily: FONT.display, fontWeight: 900, fontSize: FS, color: p.ink, letterSpacing: 0, fontVariantNumeric: 'tabular-nums',
};

export const SpeedShot: React.FC = () => {
  const frame = useCurrentFrame();
  const lg = logValueAt(frame);
  const v = Math.pow(10, lg);
  const vy = velocity((f) => logValueAt(f) * PX_PER_DECADE, frame); // tape speed, px / frame

  // entrances (inside / just after the 16 f wipe)
  const labelIn = ease.quiet(prog(frame, 6, 9));
  const ruleIn = ease.expo(prog(frame, 5, 14));
  const numIn = ease.quiet(prog(frame, 8, 10));
  const triIn = ease.quiet(prog(frame, 12, 8));
  // lock
  const tilde = ease.expo(prog(frame, B.lockAt, B.lockFx));
  const claim = ease.expo(prog(frame, B.claimIn, B.claimDur));
  const foot = ease.quiet(prog(frame, B.footAt, 12));

  // units wheel: clicks into a detent while the reading is slow, turns freely while it races
  // (a real odometer at speed is a blur of turning wheels, so the sprint reads "mid-roll").
  const rate = Math.abs(valueAt(frame + 0.5) - valueAt(frame - 0.5)); // units / frame
  const free = Math.min(1, Math.max(0, (rate - 0.6) / 1.4));
  const p0 = frame >= B.lockAt ? 113 : detent(v) + (v - detent(v)) * free; // locked: exact constant

  return (
    <AbsoluteFill>
      <SceneLabel
        p={p}
        frame={frame}
        n="02"
        label="SPEED"
        right={
          <span style={{ fontFamily: FONT.mono, fontWeight: 500, fontSize: 20, lineHeight: 1, letterSpacing: TYPE.labelTracking, color: p.muted }}>
            LOG SCALE
          </span>
        }
      />

      {/* ── readout: a ruled window (like the style frame's stat cell); the odometer wheels turn
          behind its two 1px rules, so a mid-roll digit is cut by a visible edge, not by nothing ── */}
      <div style={{ position: 'absolute', left: RX, top: WIN_TOP - 44, ...labelStyle(p, 22), opacity: labelIn }}>Render speed</div>
      <div style={{ position: 'absolute', left: RX, width: READ_W, top: WIN_TOP - 1, height: 1, background: p.line2, transformOrigin: 'left', transform: `scaleX(${ruleIn.toFixed(4)})` }} />
      <div style={{ position: 'absolute', left: RX, width: READ_W, top: WIN_TOP + FS, height: 1, background: p.line2, transformOrigin: 'left', transform: `scaleX(${ruleIn.toFixed(4)})` }} />

      <div style={{ position: 'absolute', left: 0, top: 0, opacity: numIn, transform: `translateY(${((1 - numIn) * 14).toFixed(2)}px)` }}>
        {/* "~" rises in at the lock */}
        <div style={{ position: 'absolute', left: RX, top: WIN_TOP, width: GLYPH_W, height: FS, overflow: 'hidden' }}>
          <div style={{ ...digitStyle, height: FS, lineHeight: `${FS}px`, transform: `translateY(${((1 - tilde) * 100).toFixed(3)}%)` }}>~</div>
        </div>
        <div style={{ position: 'absolute', left: ODO_X, top: WIN_TOP }}>
          <Odometer p0={p0} columns={3} pitch={PITCH} glyphW={GLYPH_W} h={FS} textStyle={digitStyle} />
        </div>
        <div style={{ position: 'absolute', left: TIMES_X, top: WIN_TOP, height: FS, lineHeight: `${FS}px`, ...digitStyle, color: p.accent }}>×</div>
      </div>

      {/* claim line: mask rise just before the lock */}
      <div style={{ position: 'absolute', left: RX, top: WIN_TOP + FS + 16, height: 32, overflow: 'hidden' }}>
        <div
          style={{
            fontFamily: FONT.mono, fontWeight: 500, fontSize: 24, lineHeight: '32px', letterSpacing: TYPE.eyebrowTracking, textTransform: 'uppercase',
            color: p.muted, whiteSpace: 'nowrap', transform: `translateY(${((1 - claim) * 110).toFixed(3)}%)`,
          }}
        >
          faster than mermaid.js
        </div>
      </div>

      {/* mandatory footnote (BRIEF: the 113x claim is a geometric mean over 25 sample diagrams):
          32 px aux size, --dim, two lines ending at canvas y ~920 (the 2-line VO3 box starts ~943) */}
      <div
        style={{
          position: 'absolute', left: RX, top: 838, fontFamily: FONT.body, fontWeight: 400, fontSize: 32, lineHeight: 1.25, color: p.dim,
          opacity: foot, transform: `translateY(${((1 - foot) * 8).toFixed(2)}px)`,
        }}
      >
        Geometric mean over
        <br />
        25 sample diagrams.
      </div>

      {/* ── tape pane ── */}
      <Pane p={p} frame={frame} x={PANE.x} y={PANE.y} w={PANE.w} h={PANE.h} start={B.paneIn} dur={16} title="RENDER SPEED" tag="LOG">
        <Tape p={p} frame={frame} lg={lg} vy={vy} />
      </Pane>

      {/* fixed pointer: accent triangle on the pane's left edge */}
      <svg width={18} height={18} style={{ position: 'absolute', left: PANE.x - 17, top: CY - 9, overflow: 'visible', opacity: triIn }}>
        <polygon points="1,1 17,9 1,17" fill={p.accent} />
      </svg>
    </AbsoluteFill>
  );
};
