// DigitRoll — odometer digit columns, adapted from video-shotcraft assets/lib/DigitRoll.tsx.
// Changes: font/colour from flowmaid tokens (pass `font`), optional `progress` override so a
// caller can drive the roll from its own timeline (e.g. a tape value) instead of `delay`.
import React from 'react';
import { Easing, interpolate, useCurrentFrame } from 'remotion';
import { FONT } from '../tokens';

const DIGITS = '0123456789';

export const DigitRoll: React.FC<{
  value: string;
  delay?: number;
  fontSize?: number;
  color: string;
  font?: string;
  weight?: number;
  progress?: number; // 0..1 overrides the delay-based timing for every column
}> = ({ value, delay = 0, fontSize = 30, color, font = FONT.mono, weight = 500, progress }) => {
  const frame = useCurrentFrame();
  const lineH = fontSize * 1.15;
  return (
    <span style={{ display: 'inline-flex', overflow: 'hidden', height: lineH, verticalAlign: 'bottom' }}>
      {value.split('').map((ch, i) => {
        const target = DIGITS.indexOf(ch);
        const cell: React.CSSProperties = { fontFamily: font, fontWeight: weight, fontSize, lineHeight: `${lineH}px`, color, fontVariantNumeric: 'tabular-nums' };
        if (target < 0) return <span key={i} style={cell}>{ch}</span>;
        const t =
          progress ??
          interpolate(frame, [delay + i * 4, delay + i * 4 + 22], [0, 1], {
            extrapolateLeft: 'clamp', extrapolateRight: 'clamp', easing: Easing.bezier(0.25, 0.8, 0.25, 1),
          });
        const offset = (10 + target) * t * lineH;
        return (
          <span key={i} style={{ display: 'inline-block', height: lineH }}>
            <span style={{ display: 'block', transform: `translateY(${-offset}px)` }}>
              {(DIGITS + DIGITS).split('').map((d, j) => (
                <span key={j} style={{ ...cell, display: 'block' }}>{d}</span>
              ))}
            </span>
          </span>
        );
      })}
    </span>
  );
};
