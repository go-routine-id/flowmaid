// Package D private helper: square L-corner brackets (focus corners / viewfinder lock).
// Adapted from demos/rhythm/beat-cut-moves/PaparazziFlash.tsx `Brackets` (square caps, no glow).
import React from 'react';

export type Rect = { x: number; y: number; w: number; h: number };

/** scale a rect around its own centre */
export const scaleRect = (r: Rect, k: number): Rect => {
  const cx = r.x + r.w / 2;
  const cy = r.y + r.h / 2;
  return { x: cx - (r.w * k) / 2, y: cy - (r.h * k) / 2, w: r.w * k, h: r.h * k };
};

/** four L corners as one SVG path; drawn inside an absolutely positioned full-size <svg>. */
export const Brackets: React.FC<{ r: Rect; color: string; arm?: number; width?: number; opacity?: number }> = ({
  r, color, arm = 34, width = 2, opacity = 1,
}) => {
  const a = Math.min(arm, r.w / 3, r.h / 3);
  const { x, y, w, h } = r;
  const d =
    `M${x} ${y + a}V${y}H${x + a} M${x + w - a} ${y}H${x + w}V${y + a} ` +
    `M${x + w} ${y + h - a}V${y + h}H${x + w - a} M${x + a} ${y + h}H${x}V${y + h - a}`;
  return <path d={d} stroke={color} strokeWidth={width} fill="none" opacity={opacity} strokeLinecap="square" strokeLinejoin="miter" />;
};
