// VoCaption — burned-in voice-over caption, adapted from video-shotcraft assets/lib/Caption.tsx.
// The library caption is a 22px mono info strip (below Q11). Here it is the narrative
// subtitle: Inter 600 52px, ink on the mode's --bg panel with a 1px --line2 frame, square
// corners, left-aligned on the gutter, bottom edge at frame y 1196 (above LinkedIn's
// overlay at y > 1215). Entrance = the approved fx.html "mask" rise (expo, 12 f);
// exit = 6 f fade. Mode-aware (Paper / Ink).
//
// Q11 note: 52px on a 1080-wide 4:5 frame is 4.8% of the frame width (Q11's 56px on a
// 1920 frame is 2.9%), so it is well above the readability bar the rule protects.
import React from 'react';
import { useCurrentFrame } from 'remotion';
import { CANVAS, CAPTION_BAND, FONT, G, ease, palette, prog, type Mode } from '../tokens';

export const CAPTION_FONT_SIZE = 52;

export const VoCaption: React.FC<{ text: string; mode: Mode; duration: number }> = ({ text, mode, duration }) => {
  const frame = useCurrentFrame(); // local to the caption's Sequence
  const p = palette(mode);
  const boxIn = ease.quiet(prog(frame, 0, 8));
  const rise = ease.expo(prog(frame, 1, 12));
  const out = 1 - prog(frame, duration - 6, 6);
  return (
    <div
      style={{
        position: 'absolute', left: G, maxWidth: 1080 - 2 * G, bottom: 1350 - (CANVAS.top + CAPTION_BAND.bottom),
        background: p.bg, border: `1px solid ${p.line2}`, padding: '16px 24px 18px', boxSizing: 'border-box',
        opacity: boxIn * out, overflow: 'hidden',
      }}
    >
      <div
        style={{
          fontFamily: FONT.body, fontWeight: 600, fontSize: CAPTION_FONT_SIZE, lineHeight: 1.16, letterSpacing: '-0.012em',
          color: p.ink, transform: `translateY(${(1 - rise) * 110}%)`,
        }}
      >
        {text}
      </div>
    </div>
  );
};
