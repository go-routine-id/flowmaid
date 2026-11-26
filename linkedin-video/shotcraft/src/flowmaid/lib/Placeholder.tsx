// Scaffold placeholder: a labelled stand-in for a shot that is not implemented yet.
// Shows the shot id, card · style key, local frame counter and the layout guides
// (SAFE content box, caption band, LinkedIn overlay line). Implementers delete the
// <Placeholder/> from their shot file once the real shot renders.
import React from 'react';
import { useCurrentFrame } from 'remotion';
import { SHOTS, type ShotId } from '../timeline';
import { CANVAS, CAPTION_BAND, FONT, LINKEDIN_OVERLAY_Y, SAFE, TYPE, displayStyle, palette } from '../tokens';
import { SceneLabel } from './Pane';

export const Guides: React.FC<{ color: string }> = ({ color }) => (
  <>
    <div style={{ position: 'absolute', left: SAFE.x, top: SAFE.top, width: SAFE.w, height: SAFE.bottom - SAFE.top, border: `1px dashed ${color}` }} />
    <div style={{ position: 'absolute', left: SAFE.x, top: CAPTION_BAND.top, width: SAFE.w, height: CAPTION_BAND.bottom - CAPTION_BAND.top, border: `1px dashed ${color}`, opacity: 0.6 }} />
    <div style={{ position: 'absolute', left: 0, right: 0, top: LINKEDIN_OVERLAY_Y - CANVAS.top, height: 0, borderTop: `2px dotted ${color}` }} />
  </>
);

export const Placeholder: React.FC<{ id: ShotId; n: string; label: string; demo: string; note?: string }> = ({ id, n, label, demo, note }) => {
  const frame = useCurrentFrame();
  const s = SHOTS[id];
  const p = palette(s.mode);
  return (
    <div style={{ position: 'absolute', inset: 0 }}>
      <Guides color={p.muted} />
      <SceneLabel p={p} frame={frame + 10} n={n} label={label} start={0} />
      <div style={{ position: 'absolute', left: SAFE.x + 24, top: 200, width: SAFE.w - 48 }}>
        <div style={{ ...displayStyle(p, 76) }}>{s.title}</div>
        <div style={{ marginTop: 28, fontFamily: FONT.mono, fontSize: 24, letterSpacing: TYPE.labelTracking, color: p.accent, textTransform: 'uppercase' }}>
          {s.card} · {s.style}
        </div>
        <div style={{ marginTop: 14, fontFamily: FONT.mono, fontSize: 20, color: p.dim }}>{demo}</div>
        {note ? <div style={{ marginTop: 22, fontFamily: FONT.body, fontSize: 26, lineHeight: 1.4, color: p.dim }}>{note}</div> : null}
        <div style={{ marginTop: 36, fontFamily: FONT.mono, fontSize: 30, color: p.ink }}>
          local {String(frame).padStart(3, '0')} / {s.duration} · abs {s.from + frame} · {s.mode.toUpperCase()}
        </div>
      </div>
    </div>
  );
};
