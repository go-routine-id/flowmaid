// Mechanical odometer for a CONTINUOUS value (package B). Unlike lib/DigitRoll (which rolls
// 0 → target once), every column here is a pure function of one wheel position, so a value
// that sprints, brakes and swings back reads like a real counter: the units wheel turns, and
// a higher wheel turns only while the wheel below it passes 9 → 0 (carry).
// Larger digits sit ABOVE the window (same direction as the speed tape), so as the value grows
// the digits travel downward, in step with the tape. Frame-pure, no state.
import React from 'react';
import { ease } from '../../tokens';

/** units-wheel position with a detent: the wheel rests on each integer and only turns in the
 *  last 40 % of every unit (Geneva-style), so slow values read as clean digits. */
export const detent = (v: number) => {
  const n = Math.floor(v);
  return n + ease.inout((v - n - 0.6) / 0.4);
};

/** wheel positions for each column (0 = units) from the units-wheel position p0. */
export const wheelPositions = (p0: number, columns: number): number[] => {
  const out = [p0];
  for (let i = 1; i < columns; i++) {
    const lower = out[i - 1];
    const base = Math.floor(lower / 10);
    const m = lower - base * 10; // lower mod 10, 0 ≤ m < 10
    out.push(base + Math.min(1, Math.max(0, m - 9)));
  }
  return out;
};

export const Odometer: React.FC<{
  p0: number; // units-wheel position (already detented by the caller if wanted)
  columns: number;
  pad?: boolean; // true: leading zeros shown ("03"); false: leading columns blank
  pitch: number; // column pitch (px)
  glyphW: number; // glyph box width (≥ pitch, so side bearings are never clipped)
  h: number; // window height = digit pitch (px)
  textStyle: React.CSSProperties; // font family / size / weight / colour
}> = ({ p0, columns, pad = false, pitch, glyphW, h, textStyle }) => {
  const wheels = wheelPositions(p0, columns);
  return (
    <span style={{ position: 'relative', display: 'inline-block', width: pitch * (columns - 1) + glyphW, height: h, verticalAlign: 'top' }}>
      {wheels.map((p, i) => {
        const left = (columns - 1 - i) * pitch;
        const k0 = Math.floor(p);
        const entries: React.ReactNode[] = [];
        for (let k = k0 - 1; k <= k0 + 2; k++) {
          const y = (p - k) * h; // larger k → above (negative y)
          if (y <= -h || y >= h) continue;
          const leadingBlank = !pad && i > 0 && k <= 0;
          const d = ((k % 10) + 10) % 10;
          entries.push(
            <span
              key={k}
              style={{
                position: 'absolute', left: 0, top: 0, width: glyphW, height: h, lineHeight: `${h}px`,
                transform: `translateY(${y.toFixed(3)}px)`, whiteSpace: 'pre', ...textStyle,
              }}
            >
              {leadingBlank ? ' ' : String(d)}
            </span>,
          );
        }
        return (
          <span key={i} style={{ position: 'absolute', left, top: 0, width: glyphW, height: h, overflow: 'hidden' }}>
            {entries}
          </span>
        );
      })}
    </span>
  );
};
