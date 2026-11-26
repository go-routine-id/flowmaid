// Shot 6 `flowcli` (abs 593-749, 156 f, Ink) — speed-ramp-freeze, style key freeze-annotate
// (references/shots/rhythm/speed-ramp-freeze.md, demo demos/rhythm/speed-ramp-freeze/FreezeAnnotateReal.tsx),
// re-skinned to the Ink playground. Hero footage: public/clips/flowcli.mp4 (real flowcli --watch in tmux).
// Canvas-local coordinates; useCurrentFrame() is local to the shot.
//
//   0-22    revealed by the mode-switch circle from the INK button. SceneLabel 05 FLOWCLI (+ TERMINAL)
//           from 4, pane rules draw 8-30, the clip (held on src 0) fades in 12-20. Status "● LIVE".
//   20-67   the clip plays 1x: the shell types `echo '    D --> N[Email buyer]' >> flow.mmd` (30-58),
//           Enter (60), flowcli --watch redraws 8 -> 9 nodes (66)
//   67      FREEZE, instant (slope 0, no ease-in): pane-body flash INK.ink 0.9 -> 0 over 3 f, accent
//           L-corners snap in at the body corners (inset 10 px, 8 f), status -> "❚❚ PAUSED"
//   70-76   everything but the new node dims (INK.bg at 0.5)
//   76-86   accent marker superellipse around "Email buyer" (one stroke, 1.08 turns, open end,
//           3.5 px, feTurbulence displacement 6, fixed seed); 86-92 arrow shaft, 92-95 head
//   90-102  annotation level with the node, in the empty lower-left of the shell pane: "redrawn on
//           save" (Inter 600 44 px) and "8 -> 9 nodes" (mono 24 px dim) rise from their lines (90 / 91);
//           102-110 hand underline
//   110-118 ink, dim and corners fade; an accent "+1" chip pins to the node's top-right (to the end)
//   118     status -> "● LIVE"; unfreeze: smootherstep over 28 f from src 86 to 121 (mp4 50-87 are
//           identical to the frozen 47, so the skip is invisible; the ease-in lands on the click on
//           Backorder at 124, its drag plays 128-143 braking; see c/flowcliMap.ts)
//   143-156 hold on src 121 (drag released, Backorder moved, edges re-routed, chip on Email buyer)
//
// Kept from the card: instant freeze (no ease-in), >= 45 f freeze (51), pause signal = flash 3 f +
// corners + LIVE -> PAUSED, 10 f marker stroke with displacement scale 6, shaft 6 f + head 3 f,
// annotation + underline, 8 f ink fade leaving a pinned comment chip, smootherstep unfreeze that
// compensates the frozen time (src 47 -> 121 in 28 f, as specified; only its start is moved past
// identical frames). Adapted (DESIGN.md 10): the rest is dimmed instead of lifting a card
// (the target is in a real video); Ink tokens; no white full-frame flash (pane body only), no
// shadows, no "FRAME HELD" caption, no reviewer avatar. Rate-linked directional blur is not used:
// the source is a TUI that redraws in discrete steps; a global directional blur would smear the
// static text around the one moving node.
import React from 'react';
import { AbsoluteFill, useCurrentFrame } from 'remotion';
import { ClipCard } from '../lib/ClipCard';
import { Pane, PANE_HEADER_H, SceneLabel } from '../lib/Pane';
import { EASE, ramp } from '../lib/Polish';
import { FONT, INK, TYPE, ease, prog } from '../tokens';
import { LCorners } from './c/Corners';
import { CARD, CLI, FREEZE, HOLE, NODE, THAW, cliSrcAt, markerPath, markerPoint } from './c/flowcliMap';

const p = INK;
const PANE = { x: 56, y: 104, w: 968, h: CARD.h + PANE_HEADER_H + 2 } as const; // 815, body 766

// annotation layout (card px). The empty lower-left of the shell pane (x < 427, the tmux divider;
// y > 320, below the shell text), level with the target so the arrow stays short. DESIGN.md said
// ~(60, 520); moved to (32, 604) so the shaft can run through the clear band between the
// "Save order" box (card y 574-614) and the Ship arrowhead (y 667), crossing only the thin
// Save order -> Ship edge (x 555) instead of the "Save order" label (measured on the render).
const NOTE = { x: 32, y: 604 };
const UNDER_Y = NOTE.y + 56;
const SUB_Y = NOTE.y + 70;
// arrow: from the end of "redrawn on save" to the marker's upper-left arc (cubic)
const tipOn = markerPoint(Math.PI + 0.45);
const A0 = { x: 392, y: NOTE.y + 26 };
const A1 = { x: tipOn.x - 9, y: tipOn.y - 8 };
const C1 = { x: 500, y: A0.y + 2 };
const C2 = { x: A1.x - 24, y: A0.y + 4 };

const Status: React.FC<{ paused: boolean }> = ({ paused }) => (
  <span
    style={{
      display: 'inline-flex', alignItems: 'center', gap: 8, fontFamily: FONT.mono, fontWeight: 500, fontSize: 16, lineHeight: 1,
      letterSpacing: TYPE.labelTracking, textTransform: 'uppercase', color: paused ? p.accent : p.dim, marginRight: 6,
    }}
  >
    {paused ? (
      <span style={{ display: 'inline-flex', gap: 3 }}>
        <span style={{ width: 4, height: 13, background: p.accent }} />
        <span style={{ width: 4, height: 13, background: p.accent }} />
      </span>
    ) : (
      <span style={{ color: p.accent, fontSize: 13 }}>●</span>
    )}
    {paused ? 'PAUSED' : 'LIVE'}
  </span>
);

/** text rising from its own line (fx "mask" rise). */
const Rise: React.FC<{ t: number; children: React.ReactNode; style?: React.CSSProperties }> = ({ t, children, style }) => (
  <div style={{ overflow: 'hidden', paddingBottom: 8, marginBottom: -8, ...style }}>
    <div style={{ transform: `translateY(${((1 - t) * 110).toFixed(2)}%)` }}>{children}</div>
  </div>
);

export const FlowcliShot: React.FC = () => {
  const frame = useCurrentFrame();
  const roughId = `fc-rough-${React.useId().replace(/[^a-zA-Z0-9]/g, '')}`;
  const frozen = frame >= FREEZE && frame < THAW;

  const clipIn = ease.quiet(prog(frame, 12, 8));

  // pause signal
  const flash = frame >= FREEZE && frame < FREEZE + 3 ? 0.9 * (1 - (frame - FREEZE) / 3) : 0;
  const out = 1 - ramp(frame, 110, 8, EASE.out); // ink / dim / corners leave together
  const snap = frame >= FREEZE ? ramp(frame, FREEZE, 8, EASE.snappy) : 0;
  const bracket = snap * out;
  const dim = 0.5 * ease.quiet(prog(frame, 70, 6)) * out;

  // marker + annotation
  const draw = ramp(frame, 76, 10, EASE.swift);
  const shaft = ramp(frame, 86, 6, EASE.out);
  const head = ramp(frame, 92, 3, EASE.out);
  const rise = ease.expo(prog(frame, 90, 12));
  const riseSub = ease.expo(prog(frame, 91, 10));
  const under = ramp(frame, 102, 8, EASE.swift);
  const showInk = frame >= 76 && out > 0.001;

  // pinned comment chip
  const pin = frame >= 110 ? ramp(frame, 110, 8, EASE.snappy) : 0;

  // arrowhead wings at A1, along the curve's end tangent
  const ang = Math.atan2(A1.y - C2.y, A1.x - C2.x);
  const wing = (s: number) =>
    `M ${A1.x.toFixed(1)} ${A1.y.toFixed(1)} L ${(A1.x - Math.cos(ang + s * 0.5) * 16).toFixed(1)} ${(A1.y - Math.sin(ang + s * 0.5) * 16).toFixed(1)}`;

  const inset = 10;
  const off = (1 - snap) * 24; // corners snap in from 24 px outside

  return (
    <AbsoluteFill>
      <SceneLabel
        p={p} frame={frame} n="05" label="FLOWCLI" start={4}
        right={<span style={{ fontFamily: FONT.mono, fontWeight: 500, fontSize: 22, lineHeight: 1, letterSpacing: TYPE.labelTracking, textTransform: 'uppercase', color: p.muted }}>TERMINAL</span>}
      />

      <Pane
        p={p} frame={frame} x={PANE.x} y={PANE.y} w={PANE.w} h={PANE.h} start={8} dur={22}
        title="FLOWCLI — FLOW.MMD" tag="--WATCH" fill="canvas" right={<Status paused={frozen} />}
      >
        <div style={{ position: 'absolute', left: 0, top: 0, width: CARD.w, height: CARD.h, overflow: 'hidden' }}>
          <div style={{ position: 'absolute', inset: 0, opacity: clipIn }}>
            <ClipCard src={CLI.src} srcW={CLI.w} srcH={CLI.h} width={CARD.w} height={CARD.h} map={cliSrcAt} background={p.canvas} />
          </div>

          {/* everything but the new node dims (4 rects around the hole) */}
          {dim > 0.001 ? (
            <div style={{ position: 'absolute', inset: 0, opacity: dim }}>
              <div style={{ position: 'absolute', left: 0, top: 0, width: CARD.w, height: HOLE.y0, background: p.bg }} />
              <div style={{ position: 'absolute', left: 0, top: HOLE.y1, width: CARD.w, height: CARD.h - HOLE.y1, background: p.bg }} />
              <div style={{ position: 'absolute', left: 0, top: HOLE.y0, width: HOLE.x0, height: HOLE.y1 - HOLE.y0, background: p.bg }} />
              <div style={{ position: 'absolute', left: HOLE.x1, top: HOLE.y0, width: CARD.w - HOLE.x1, height: HOLE.y1 - HOLE.y0, background: p.bg }} />
            </div>
          ) : null}

          {/* marker ink: superellipse, arrow, underline (hand-drawn wobble from a fixed-seed displacement) */}
          {showInk ? (
            <svg width={CARD.w} height={CARD.h} style={{ position: 'absolute', left: 0, top: 0, opacity: out, pointerEvents: 'none' }}>
              <defs>
                <filter id={roughId} filterUnits="userSpaceOnUse" x={0} y={0} width={CARD.w} height={CARD.h}>
                  <feTurbulence type="fractalNoise" baseFrequency="0.022" numOctaves={2} seed={11} result="n" />
                  <feDisplacementMap in="SourceGraphic" in2="n" scale={6} xChannelSelector="R" yChannelSelector="G" />
                </filter>
              </defs>
              <g filter={`url(#${roughId})`} fill="none" stroke={p.accent} strokeLinecap="round" strokeLinejoin="round">
                <path d={markerPath} strokeWidth={3.5} pathLength={1} strokeDasharray="1 1" strokeDashoffset={1 - draw} transform={`rotate(-3 ${((NODE.x0 + NODE.x1) / 2).toFixed(1)} ${((NODE.y0 + NODE.y1) / 2).toFixed(1)})`} />
                {shaft > 0 ? (
                  <path d={`M ${A0.x} ${A0.y} C ${C1.x} ${C1.y} ${C2.x.toFixed(1)} ${C2.y.toFixed(1)} ${A1.x.toFixed(1)} ${A1.y.toFixed(1)}`} strokeWidth={3} pathLength={1} strokeDasharray="1 1" strokeDashoffset={1 - shaft} />
                ) : null}
                {head > 0 ? (
                  <>
                    <path d={wing(1)} strokeWidth={3} pathLength={1} strokeDasharray="1 1" strokeDashoffset={1 - head} />
                    <path d={wing(-1)} strokeWidth={3} pathLength={1} strokeDasharray="1 1" strokeDashoffset={1 - head} />
                  </>
                ) : null}
                {under > 0 ? (
                  <path
                    d={`M ${NOTE.x} ${UNDER_Y + 2} C ${NOTE.x + 110} ${UNDER_Y - 3} ${NOTE.x + 240} ${UNDER_Y + 1} ${NOTE.x + 346} ${UNDER_Y - 4}`}
                    strokeWidth={3} pathLength={1} strokeDasharray="1 1" strokeDashoffset={1 - under}
                  />
                ) : null}
              </g>
            </svg>
          ) : null}

          {/* annotation */}
          {showInk && frame >= 90 ? (
            <div style={{ position: 'absolute', left: NOTE.x, top: NOTE.y, opacity: out }}>
              <Rise t={rise}>
                <div style={{ fontFamily: FONT.body, fontWeight: 600, fontSize: 44, lineHeight: 1.1, letterSpacing: '-0.012em', color: p.ink, whiteSpace: 'nowrap' }}>
                  redrawn on save
                </div>
              </Rise>
            </div>
          ) : null}
          {showInk && frame >= 91 ? (
            <div style={{ position: 'absolute', left: NOTE.x + 2, top: SUB_Y, opacity: out }}>
              <Rise t={riseSub}>
                <div style={{ fontFamily: FONT.mono, fontWeight: 500, fontSize: 24, lineHeight: 1.2, color: p.dim, whiteSpace: 'nowrap' }}>8 -&gt; 9 nodes</div>
              </Rise>
            </div>
          ) : null}

          {/* pinned "+1" comment chip at the node's top-right (stays to the end) */}
          {pin > 0.001 ? (
            <div
              style={{
                position: 'absolute', left: NODE.x1 - 12, top: NODE.y0 - 26, padding: '5px 7px', background: p.accent, color: p.accentInk,
                fontFamily: FONT.mono, fontWeight: 600, fontSize: 17, lineHeight: 1, letterSpacing: '0.02em',
                transform: `scale(${(0.6 + 0.4 * pin).toFixed(4)})`, transformOrigin: '0% 100%', opacity: Math.min(1, pin * 1.6),
              }}
            >
              +1
            </div>
          ) : null}

          {/* pause signal: pane-body flash + viewfinder corners at the body corners */}
          {flash > 0 ? <div style={{ position: 'absolute', inset: 0, background: p.ink, opacity: flash }} /> : null}
          {bracket > 0.001 ? (
            <LCorners
              rect={{ x0: inset - off, y0: inset - off, x1: CARD.w - inset + off, y1: CARD.h - inset + off }}
              color={p.accent} arm={40} stroke={2} opacity={Math.min(1, snap * 1.5) * out} width={CARD.w} height={CARD.h}
            />
          ) : null}
        </div>
      </Pane>
    </AbsoluteFill>
  );
};
