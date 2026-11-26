// Package D private helper: the approved fx.html "mask rise" (translateY 110% -> 0 inside a
// clip box exactly one line box tall), so nothing of the text shows before its cue.
import React from 'react';

export const MaskRise: React.FC<{
  t: number; // 0 hidden .. 1 in place (already eased)
  height: number; // line box height of the content (px); the mask is exactly this tall
  style?: React.CSSProperties;
  children: React.ReactNode;
}> = ({ t, height, style, children }) => (
  <div style={{ position: 'relative', height, overflow: 'hidden', ...style }}>
    <div style={{ height, display: 'flex', alignItems: 'center', transform: `translateY(${((1 - t) * 110).toFixed(3)}%)`, opacity: t > 0 ? 1 : 0 }}>
      {children}
    </div>
  </div>
);
