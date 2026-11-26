// Pane — the playground's framed panel whose 1px lines draw themselves (approved
// fx.html `drawPane`): top → right → bottom → left in turn (expo per segment), then the
// fill and the 48px header fade in. Square corners, no shadow. Canvas-local coordinates.
//
// Use it for every framed surface (code pane, SVG pane, clip panes, tape, cargo chip) so
// the "structure comes from lines" entrance stays one voice across the film.
import React from 'react';
import { FONT, TYPE, dotBackground, ease, prog, type Palette } from '../tokens';

export const PANE_HEADER_H = 48;
const SEG = [0, 0.32, 0.5, 0.82, 1];

export const Pane: React.FC<{
  p: Palette;
  frame: number; // shot-local frame
  x: number; y: number; w: number; h: number;
  start?: number; // local frame the lines start drawing
  dur?: number; // line-draw duration (fx: 22 f)
  title?: string; // header left (mono uppercase); omit for a header-less pane
  tag?: string; // header right accent tag
  tagActive?: boolean; // tag filled with accent (confirm state)
  right?: React.ReactNode; // extra header-right content (status text etc.)
  fill?: 'panel' | 'canvas' | 'bg' | 'none' | string;
  lineColor?: string; // default p.line2
  bodyStyle?: React.CSSProperties;
  children?: React.ReactNode; // body content, positioned relative to the body box
}> = ({ p, frame, x, y, w, h, start = 0, dur = 22, title, tag, tagActive, right, fill = 'panel', lineColor, bodyStyle, children }) => {
  const q = prog(frame, start, dur);
  const seg = (i: number) => ease.expo((q - SEG[i]) / (SEG[i + 1] - SEG[i]));
  const f = ease.quiet(prog(frame, start + dur * 0.7, dur * 0.3 + 7.5));
  const lc = lineColor ?? p.line2;
  const hasHeader = title !== undefined || tag !== undefined;
  const fillStyle: React.CSSProperties =
    fill === 'none' ? {} : fill === 'panel' ? { background: p.panel } : fill === 'bg' ? { background: p.bg } : fill === 'canvas' ? dotBackground(p) : { background: fill };
  const line = (style: React.CSSProperties): React.CSSProperties => ({ position: 'absolute', background: lc, ...style });
  return (
    <div style={{ position: 'absolute', left: x, top: y, width: w, height: h }}>
      <div style={{ position: 'absolute', inset: 0, opacity: f, ...fillStyle }} />
      {hasHeader ? (
        <div
          style={{
            position: 'absolute', left: 1, right: 1, top: 1, height: PANE_HEADER_H - 1, display: 'flex', alignItems: 'center', gap: 12,
            padding: '0 18px', borderBottom: `1px solid ${lc}`, background: p.bg, opacity: f,
            fontFamily: FONT.mono, fontWeight: 500, fontSize: 17, letterSpacing: TYPE.labelTracking, textTransform: 'uppercase', color: p.dim,
          }}
        >
          <span>{title}</span>
          <span style={{ flex: 1 }} />
          {right}
          {tag ? (
            <span
              style={{
                padding: '6px 10px', background: tagActive ? p.accent : p.accentSoft, color: tagActive ? p.accentInk : p.accent,
                fontSize: 16, lineHeight: 1, letterSpacing: TYPE.labelTracking,
              }}
            >
              {tag}
            </span>
          ) : null}
        </div>
      ) : null}
      <div style={{ position: 'absolute', left: 1, right: 1, top: hasHeader ? PANE_HEADER_H : 1, bottom: 1, overflow: 'hidden', ...bodyStyle }}>{children}</div>
      {/* the four rules draw in turn: top (from left), right (from top), bottom (from right), left (from bottom) */}
      <div style={line({ left: 0, right: 0, top: 0, height: 1, transformOrigin: 'left', transform: `scaleX(${seg(0)})` })} />
      <div style={line({ top: 0, bottom: 0, right: 0, width: 1, transformOrigin: 'top', transform: `scaleY(${seg(1)})` })} />
      <div style={line({ left: 0, right: 0, bottom: 0, height: 1, transformOrigin: 'right', transform: `scaleX(${seg(2)})` })} />
      <div style={line({ top: 0, bottom: 0, left: 0, width: 1, transformOrigin: 'bottom', transform: `scaleY(${seg(3)})` })} />
    </div>
  );
};

/** Scene label row ("01" tag + label), as in the style frames. Canvas-local. */
export const SceneLabel: React.FC<{ p: Palette; frame: number; n: string; label: string; start?: number; x?: number; y?: number; right?: React.ReactNode }> = ({
  p, frame, n, label, start = 4, x = 56, y = 44, right,
}) => {
  const o = ease.quiet(prog(frame, start, 9));
  return (
    <div style={{ position: 'absolute', left: x, right: 56, top: y, display: 'flex', alignItems: 'center', gap: 14, opacity: o }}>
      <span style={{ padding: '7px 11px', background: p.accentSoft, color: p.accent, fontFamily: FONT.mono, fontWeight: 500, fontSize: 20, lineHeight: 1, letterSpacing: TYPE.labelTracking }}>{n}</span>
      <span style={{ fontFamily: FONT.mono, fontWeight: 500, fontSize: 22, lineHeight: 1, letterSpacing: TYPE.labelTracking, textTransform: 'uppercase', color: p.dim }}>{label}</span>
      <span style={{ flex: 1 }} />
      {right}
    </div>
  );
};
