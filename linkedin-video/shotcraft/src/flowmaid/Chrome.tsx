// Chrome — the playground frame around every shot, per mode (fx.html):
//   header 96px: "flowmaid" (Archivo 900 44px) · accent eyebrow "—— PURE RUST · ZERO DEPS" ·
//                INK | PAPER toggle with a sliding ink knob; 1px --line2 bottom rule
//   canvas: dotted whiteboard (--canvas + --dot, 28px grid); shots render INSIDE it in
//           canvas-local coordinates (0,0 = frame (0,96)), clipped to 1080 × 1200
//   footer 54px: mono uppercase muted strip; 1px --line2 top rule
// Owned by the scaffold. Shot packages never edit this file; Main.tsx feeds it the
// timeline-derived props (knob, header progress, wordmark visibility).
import React from 'react';
import { CANVAS, FONT, FOOTER_H, FOOTER_LEFT, FOOTER_RIGHT, G, HEADER, HEADER_H, TYPE, dotBackground, ease, palette, type Mode } from './tokens';

const Toggle: React.FC<{ mode: Mode; knob: number }> = ({ mode, knob }) => {
  const p = palette(mode);
  // segment geometry (measured from fx.html at 16px mono, .16em tracking, 11px/15px padding)
  const W_INK = 76;
  const W_PAPER = 100;
  const x = knob * W_INK; // knob left: 0 (INK) → W_INK (PAPER)
  const w = W_INK + (W_PAPER - W_INK) * knob;
  const seg = (label: string, active: boolean, width: number): React.ReactNode => (
    <span
      style={{
        position: 'relative', width, height: 38, display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
        fontFamily: FONT.mono, fontWeight: 500, fontSize: 16, letterSpacing: TYPE.eyebrowTracking, textTransform: 'uppercase',
        color: active ? p.bg : p.muted,
      }}
    >
      {label}
    </span>
  );
  return (
    <span style={{ position: 'relative', display: 'inline-flex', border: `1px solid ${p.line2}` }}>
      <i style={{ position: 'absolute', top: 0, bottom: 0, left: x, width: w, background: p.ink }} />
      {seg('Ink', knob < 0.5, W_INK)}
      {seg('Paper', knob >= 0.5, W_PAPER)}
    </span>
  );
};

export const ModeLayer: React.FC<{
  mode: Mode;
  knob: number; // 0 = INK selected … 1 = PAPER selected (already eased)
  headerProgress?: number; // 0..1 header/footer draw-in (opening only)
  wordmarkVisible?: boolean;
  clipPath?: string; // transition reveal (wipe / circle), applied to the whole layer
  children?: React.ReactNode; // canvas content (canvas-local coordinates)
}> = ({ mode, knob, headerProgress = 1, wordmarkVisible = true, clipPath, children }) => {
  const p = palette(mode);
  const hp = headerProgress;
  const bar = ease.quiet(hp); // bar fill
  const rule = ease.expo(hp); // rules draw L→R
  const eb = HEADER.eyebrow;
  const ebChars = Math.floor(eb.length * Math.max(0, Math.min(1, (hp - 0.2) / 0.7)));
  return (
    <div style={{ position: 'absolute', inset: 0, background: p.bg, clipPath, WebkitClipPath: clipPath, isolation: 'isolate' }}>
      {/* isolation: each layer is its own stacking context, so z-index inside a shot can never
          leak above the other layer during a wipe / mode-switch reveal. */}
      {/* full-frame dotted canvas: visible behind the bars while they draw in (opening) */}
      <div style={{ position: 'absolute', inset: 0, ...dotBackground(p) }} />

      {/* shot content, clipped to the canvas box */}
      <div style={{ position: 'absolute', left: 0, top: CANVAS.top, width: CANVAS.w, height: CANVAS.h, overflow: 'hidden' }}>{children}</div>

      {/* header */}
      <div style={{ position: 'absolute', left: 0, right: 0, top: 0, height: HEADER_H }}>
        <div style={{ position: 'absolute', inset: 0, background: p.bg, opacity: bar }} />
        <div style={{ position: 'absolute', left: 0, right: 0, bottom: 0, height: 1, background: p.line2, transformOrigin: 'left', transform: `scaleX(${rule})` }} />
        <div style={{ position: 'absolute', left: G, right: G, top: 0, bottom: 0, display: 'flex', alignItems: 'center', gap: 22 }}>
          <span
            style={{
              fontFamily: FONT.display, fontWeight: 900, fontSize: HEADER.wordmarkSize, lineHeight: 1, letterSpacing: TYPE.displayTracking,
              color: p.ink, opacity: wordmarkVisible ? 1 : 0,
            }}
          >
            flowmaid
          </span>
          <span
            style={{
              display: 'inline-flex', alignItems: 'center', gap: 14, fontFamily: FONT.mono, fontWeight: 500, fontSize: 17, lineHeight: 1,
              letterSpacing: TYPE.eyebrowTracking, textTransform: 'uppercase', color: p.accent, opacity: hp > 0 ? 1 : 0,
            }}
          >
            <span style={{ width: 40, height: 1, background: p.accent, transformOrigin: 'left', transform: `scaleX(${rule})` }} />
            <span style={{ whiteSpace: 'pre' }}>{eb.slice(0, ebChars)}</span>
          </span>
          <span style={{ flex: 1 }} />
          <span style={{ opacity: ease.quiet((hp - 0.4) / 0.6) }}>
            <Toggle mode={mode} knob={knob} />
          </span>
        </div>
      </div>

      {/* footer */}
      <div style={{ position: 'absolute', left: 0, right: 0, bottom: 0, height: FOOTER_H }}>
        <div style={{ position: 'absolute', inset: 0, background: p.bg, opacity: bar }} />
        <div style={{ position: 'absolute', left: 0, right: 0, top: 0, height: 1, background: p.line2, transformOrigin: 'left', transform: `scaleX(${rule})` }} />
        <div
          style={{
            position: 'absolute', left: G, right: G, top: 0, bottom: 0, display: 'flex', alignItems: 'center', gap: 26,
            fontFamily: FONT.mono, fontWeight: 400, fontSize: 17, lineHeight: 1, letterSpacing: TYPE.labelTracking, textTransform: 'uppercase',
            color: p.muted, opacity: ease.quiet((hp - 0.3) / 0.7),
          }}
        >
          {FOOTER_LEFT.map((t, i) => (
            <React.Fragment key={t}>
              {i > 0 ? <span>·</span> : null}
              <span>{t}</span>
            </React.Fragment>
          ))}
          <span style={{ flex: 1 }} />
          <span>{FOOTER_RIGHT}</span>
        </div>
      </div>
    </div>
  );
};
