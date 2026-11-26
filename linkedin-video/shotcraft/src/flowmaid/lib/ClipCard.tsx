// ClipCard (flowmaid edition) — adapted from video-shotcraft assets/lib/ClipCard.tsx.
// Wraps a real mp4 clip so shot-card camera grammars can drive it.
//
// Changes vs the library original (brief: square corners, 1px rules, no shadows):
//  - any aspect ratio (width × height, `srcW`/`srcH` of the clip), radius 0, no shadow,
//    1px border from the palette, no caption bar (panes carry headers: see Pane.tsx);
//  - `camera`: zoom + focus point in SOURCE pixels (zoom 1 = whole clip fitted to width);
//  - `map`: optional output→source frame remap (freeze / speed ramps / segment jumps),
//    implemented with <Freeze> + <OffthreadVideo> (frame-exact, deterministic);
//  - the library's crossfade loop is kept (`loopDurationInFrames`) for clips shorter than
//    their shot, but none of our clips need it (every remap holds the last frame instead).
// Always muted: the master is silent.
import React from 'react';
import { Freeze, OffthreadVideo, Sequence, interpolate, staticFile, useCurrentFrame, useVideoConfig } from 'remotion';

export type ClipCamera = {
  zoom: number; // 1 = whole clip fitted to the card width; 2 = twice as close
  cx: number; // focus point, source px (default: clip centre)
  cy: number;
};

const LoopLayer: React.FC<{ src: string; startFrom: number; fadeIn: number; fadeOut: number; step: number; style: React.CSSProperties }> = ({
  src, startFrom, fadeIn, fadeOut, step, style,
}) => {
  const frame = useCurrentFrame();
  let envelope: number;
  if (frame <= fadeIn) envelope = fadeIn === 0 ? 1 : interpolate(frame, [0, fadeIn], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' });
  else if (frame <= step) envelope = 1;
  else envelope = interpolate(frame, [step, step + fadeOut], [1, 0], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' });
  return <OffthreadVideo src={staticFile(src)} muted startFrom={startFrom} style={{ ...style, opacity: Math.max(0, Math.min(1, envelope)) }} />;
};

/** Output→source frame remap. `map(localFrame)` returns the clip frame to show. */
export const RemapVideo: React.FC<{ src: string; map: (localFrame: number) => number; style?: React.CSSProperties }> = ({ src, map, style }) => {
  const frame = useCurrentFrame();
  const srcFrame = Math.max(0, Math.round(map(frame)));
  return (
    <Freeze frame={srcFrame}>
      <OffthreadVideo src={staticFile(src)} muted style={style} />
    </Freeze>
  );
};

export const ClipCard: React.FC<{
  src: string; // path under public/, e.g. 'clips/web.mp4'
  srcW: number; // clip pixel size
  srcH: number;
  width: number; // card size on screen (px)
  height: number;
  camera?: Partial<ClipCamera>;
  map?: (localFrame: number) => number; // remap (freeze / ramp); omit for 1× playback
  startFrom?: number; // 1× playback trim (ignored when `map` is given)
  loopDurationInFrames?: number;
  loopCrossfadeInFrames?: number;
  durationInFrames?: number; // enclosing shot duration (loop layer count)
  border?: string; // e.g. `1px solid ${p.line2}`; default none (Pane draws the frame)
  background?: string;
  style?: React.CSSProperties;
  children?: React.ReactNode; // overlays in CARD pixels, drawn above the video (not zoomed)
}> = ({
  src, srcW, srcH, width, height, camera, map, startFrom = 0, loopDurationInFrames, loopCrossfadeInFrames = 8,
  durationInFrames, border, background = '#000', style, children,
}) => {
  const { durationInFrames: compDuration } = useVideoConfig();
  const zoom = camera?.zoom ?? 1;
  const cx = camera?.cx ?? srcW / 2;
  const cy = camera?.cy ?? srcH / 2;
  // base scale fits the clip to the card width; the camera zooms around (cx, cy) and keeps
  // that point at the card centre (clamped so the card never shows past the clip edge).
  const s = (width / srcW) * zoom;
  const { tx, ty } = camTranslate(width, height, srcW, srcH, s, cx, cy);
  const videoStyle: React.CSSProperties = { position: 'absolute', left: 0, top: 0, width: srcW, height: srcH, display: 'block' };

  let video: React.ReactNode;
  const step = loopDurationInFrames ? loopDurationInFrames - startFrom - loopCrossfadeInFrames : 0;
  const canLoop = !map && !!loopDurationInFrames && loopDurationInFrames > loopCrossfadeInFrames && step >= loopCrossfadeInFrames;
  if (map) {
    video = <RemapVideo src={src} map={map} style={videoStyle} />;
  } else if (canLoop) {
    const n = Math.max(1, Math.ceil((durationInFrames ?? compDuration) / step));
    video = Array.from({ length: n }, (_, i) => (
      <Sequence key={i} from={i * step} durationInFrames={loopDurationInFrames} layout="none">
        <LoopLayer src={src} startFrom={startFrom} fadeIn={i === 0 ? 0 : loopCrossfadeInFrames} fadeOut={loopCrossfadeInFrames} step={step} style={videoStyle} />
      </Sequence>
    ));
  } else {
    video = <OffthreadVideo src={staticFile(src)} muted startFrom={startFrom} style={videoStyle} />;
  }

  return (
    <div style={{ position: 'relative', width, height, overflow: 'hidden', background, border, boxSizing: 'border-box', ...style }}>
      <div style={{ position: 'absolute', left: 0, top: 0, width: srcW, height: srcH, transformOrigin: '0 0', transform: `translate(${tx}px, ${ty}px) scale(${s})` }}>
        {video}
      </div>
      {children}
    </div>
  );
};

// Keep (cx, cy) at the card centre, but never show outside the clip: when the scaled clip
// is larger than the card on an axis, the translation is clamped so the clip covers it.
function camTranslate(width: number, height: number, srcW: number, srcH: number, s: number, cx: number, cy: number) {
  let tx = width / 2 - cx * s;
  let ty = height / 2 - cy * s;
  const w = srcW * s;
  const h = srcH * s;
  tx = w >= width ? Math.min(0, Math.max(width - w, tx)) : (width - w) / 2;
  ty = h >= height ? Math.min(0, Math.max(height - h, ty)) : ty;
  return { tx, ty };
}

/** Map a SOURCE-pixel point to CARD pixels for a given camera (for overlays: brackets, circles). */
export const clipToCard = (pt: { x: number; y: number }, o: { srcW: number; srcH: number; width: number; height: number; camera?: Partial<ClipCamera> }) => {
  const zoom = o.camera?.zoom ?? 1;
  const cx = o.camera?.cx ?? o.srcW / 2;
  const cy = o.camera?.cy ?? o.srcH / 2;
  const s = (o.width / o.srcW) * zoom;
  const { tx, ty } = camTranslate(o.width, o.height, o.srcW, o.srcH, s, cx, cy);
  return { x: tx + pt.x * s, y: ty + pt.y * s, scale: s };
};
