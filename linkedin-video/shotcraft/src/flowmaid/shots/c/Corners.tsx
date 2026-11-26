// Package C private helper: four accent L-corners around a rectangle, drawn in the
// coordinates of the element that contains them (card px). Square caps, constant
// on-screen stroke and arm length (they do not scale with a camera; only their
// positions ride it). No glow, no shadow.
import React from 'react';

export type Rect = { x0: number; y0: number; x1: number; y1: number };

export const LCorners: React.FC<{
  rect: Rect;
  color: string;
  arm?: number; // on-screen arm length (px)
  stroke?: number; // on-screen stroke (px)
  opacity?: number;
  width: number; // size of the containing box (the SVG covers it)
  height: number;
}> = ({ rect, color, arm = 24, stroke = 2, opacity = 1, width, height }) => {
  if (opacity <= 0.001) return null;
  const { x0, y0, x1, y1 } = rect;
  const a = Math.min(arm, Math.abs(x1 - x0) / 2, Math.abs(y1 - y0) / 2);
  const f = (n: number) => n.toFixed(2);
  return (
    <svg width={width} height={height} style={{ position: 'absolute', left: 0, top: 0, overflow: 'visible', opacity, pointerEvents: 'none' }}>
      <g fill="none" stroke={color} strokeWidth={stroke} strokeLinecap="square">
        <path d={`M${f(x0)} ${f(y0 + a)} V${f(y0)} H${f(x0 + a)}`} />
        <path d={`M${f(x1 - a)} ${f(y0)} H${f(x1)} V${f(y0 + a)}`} />
        <path d={`M${f(x1)} ${f(y1 - a)} V${f(y1)} H${f(x1 - a)}`} />
        <path d={`M${f(x0 + a)} ${f(y1)} H${f(x0)} V${f(y1 - a)}`} />
      </g>
    </svg>
  );
};
