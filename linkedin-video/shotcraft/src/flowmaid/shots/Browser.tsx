// Shot 5 `browser` (abs 451-601, 150 f) — crash-zoom-punch, bounce variant
// (references/shots/camera/crash-zoom-punch.md, demo demos/camera/crash-zoom-punch/CrashZoomReal.tsx),
// re-skinned to the Paper playground. Hero footage: public/clips/web.mp4 (the real flowmaid-web
// playground). Canvas-local coordinates; useCurrentFrame() is local to the shot.
//
//   0-16    revealed by the house wipe. SceneLabel 04 FLOWMAID-WEB (+ WEBASSEMBLY tag) from 4,
//           pane rules draw 2-24, the clip fades in 6-16 (whole playground, zoom 1).
//   16-46   the full view holds 30 f (card: >= 30 f to establish), creeping 1 -> 1.03.
//   22-38   four accent L-corners (2 px, 24 px arms) contract from 150 to 14 src px outside the
//           region the crash lands on (src x 772-1500, y 96-568: Request, Auth? over its whole
//           travel, 401 Denied, Cache). They live in source coordinates and ride the camera: named
//           in the wide shot first, they end as a viewfinder ~10 px inside the close-up's edges.
//   46-52   anticipation: zoom x0.97 back to the full view, corners breathe out 8 src px
//   52-58   CRASH, 6 f: log-space strong ease-in (EASE.exit) to 2.06 x 1.03, the focus point
//           moves monotonically to the card centre. Time-sampled motion blur on these frames
//           only (equal-weight sub-frame layers of the same video frame, echo spacing <= 6 px,
//           <= 44 layers, <= 4 px smear per layer on the fastest frames).
//   58      lands on the frame where the cursor presses Auth? (src 52; src = local - 6)
//   58-70   damped-cosine settle to 2.06 (one visible rebound, 3 %); corners fade 70-78
//   59-123  the real drag in close-up (Auth? travels left, its three edges re-route every frame;
//           motion src 60-112 = local 66-118); creep x1.01, left edge pinned
//   123     release (src 117: the node toolbar pops back)
//   123-150 settled hold; source capped at src 134, so from local 124 nothing changes in view
//           (src 118-134 are identical in the close-up; calm for the knob slide 141-150)
// Fixer round 1: the anticipation / crash / drag run 12 f later than in the first cut (crash at
// 40 -> 52), taken from the post-release hold, so the full view holds 30 f before the anticipation.
//
// Kept from the card: wide hold that names the target, anticipation, 6 f ease-in push with the
// speed peak on the landing frame, small bounce, blur only on the push, demo shutter/echo rules.
// Removed for the brief (DESIGN.md 10): ember stage, back-tilt, exposure flash, periphery
// darkening, rim light, right-column readout. Deviations from DESIGN.md 8.5 (see c/browserCam.ts):
// rest zoom 2.06 at a left-pinned centre instead of 2.13 at (1080, 320); creep 3 % / anticipation
// 3 % instead of 2 % / 4 %; source frames capped at 150 instead of 155.
import React from 'react';
import { AbsoluteFill, useCurrentFrame } from 'remotion';
import { ClipCard } from '../lib/ClipCard';
import { Pane, PANE_HEADER_H, SceneLabel } from '../lib/Pane';
import { EASE, mix, ramp } from '../lib/Polish';
import { PAPER, ease, prog, tagStyle } from '../tokens';
import { LCorners } from './c/Corners';
import { ANT, CARD, CORNERS_AT, HIT, PUSH, SHUTTER, TARGET, WEB, blurSamplesAt, blurSmearAt, camAt, toCard, webSrcFrame } from './c/browserCam';

const p = PAPER;
const PANE = { x: 56, y: 160, w: 968, h: CARD.h + PANE_HEADER_H + 2 } as const; // 696

/** the clip + its camera-riding corners at (fractional) camera time `t`. */
const World: React.FC<{ t: number; lock: number; breath: number; cornerFade: number }> = ({ t, lock, breath, cornerFade }) => {
  const cam = camAt(t);
  const pad = mix(150, 14, lock) + 8 * breath; // source px
  const a = toCard({ x: TARGET.x0 - pad, y: TARGET.y0 - pad }, t);
  const b = toCard({ x: TARGET.x1 + pad, y: TARGET.y1 + pad }, t);
  const cornerOpacity = Math.min(1, lock * 4) * cornerFade;
  return (
    <ClipCard src={WEB.src} srcW={WEB.w} srcH={WEB.h} width={CARD.w} height={CARD.h} camera={cam} map={webSrcFrame} background={p.canvas}>
      <LCorners rect={{ x0: a.x, y0: a.y, x1: b.x, y1: b.y }} color={p.accent} arm={24} stroke={2} opacity={cornerOpacity} width={CARD.w} height={CARD.h} />
    </ClipCard>
  );
};

export const BrowserShot: React.FC = () => {
  const frame = useCurrentFrame();

  const lock = frame < CORNERS_AT ? 0 : ramp(frame, CORNERS_AT, 16, EASE.snappy);
  const breath = ramp(frame, ANT, PUSH - ANT, EASE.smooth) * (1 - ramp(frame, PUSH, 4, EASE.exit));
  const cornerFade = 1 - ease.quiet(prog(frame, HIT + 12, 8));
  const clipIn = ease.quiet(prog(frame, 6, 10));

  const n = blurSamplesAt(frame);
  const smear = blurSmearAt(frame, n);

  return (
    <AbsoluteFill>
      <SceneLabel p={p} frame={frame} n="04" label="FLOWMAID-WEB" start={4} right={<span style={tagStyle(p)}>WEBASSEMBLY</span>} />

      <Pane p={p} frame={frame} x={PANE.x} y={PANE.y} w={PANE.w} h={PANE.h} start={2} dur={22} title="OPENSOURCE.GO-ROUTINE.COM/FLOWMAID-WEB" tag="WASM" fill="canvas">
        <div style={{ position: 'absolute', left: 0, top: 0, width: CARD.w, height: CARD.h, opacity: clipIn, overflow: 'hidden' }}>
          {n > 1 ? (
            Array.from({ length: n }, (_, i) => (
              <div
                key={i}
                style={{
                  position: 'absolute', left: 0, top: 0, width: CARD.w, height: CARD.h, opacity: 1 / (i + 1),
                  filter: smear > 0.3 ? `blur(${smear.toFixed(2)}px)` : undefined,
                }}
              >
                <World t={frame - (SHUTTER * i) / (n - 1)} lock={lock} breath={breath} cornerFade={cornerFade} />
              </div>
            ))
          ) : (
            <World t={frame} lock={lock} breath={breath} cornerFade={cornerFade} />
          )}
        </div>
      </Pane>
    </AbsoluteFill>
  );
};
