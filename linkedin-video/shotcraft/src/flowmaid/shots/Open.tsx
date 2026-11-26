// Shot 1 `open` (abs 0-90) — brand-ink-open (references/shots/opening/brand-ink-open.md,
// demo demos/typography/brand-ink-open/BrandInkOpen.tsx), re-skinned to flowmaid-web Paper.
//
//   0-9 / 8-18   accent crosshair (64 px, 3 px, butt caps) draws vertical then horizontal; fades 24-34
//   10-43        "flowmaid" letterpress, glyph i: delay 10 + 3i, 12 f, scale 1.6 -> 1 (origin centre
//                bottom) + blur 6 -> 0 + opacity, accent 2 px glint dash under the glyph as it lands
//   28-43.4      kicker "OPEN-SOURCE RUST CRATE" types at 0.7 f/char, accent block cursor blinks
//                (2 f on / 2 f off) and stops at 74; kicker fades 74-78
//   46-74        brand hold (R1): nothing moves except the card's cursor blink
//   74-87        dock (ease.inout): the wordmark moves + shrinks into the header slot (Q9: land in a
//                real slot instead of the card's fade exit). At local 87 it sits exactly where Chrome
//                draws its own 44 px wordmark; Chrome shows it from abs 88 and this overlay ends.
//
// Card pitfall parameters kept: crosshair timings, glyph delay/duration/scale/blur, 0.7 f/char,
// 2 f cursor blink stopping at 74, a full 1 s brand hold.
// Adapted (DESIGN.md section 10): Archivo 900 instead of a serif, accent instead of amber, the exit
// fade replaced by the dock.
//
// Layout note (measured in the render browser): per-glyph inline blocks of Archivo 900 sit within
// 0.08 px of the kerned single text node Chrome uses in the header, and the 44 px layout is an exact
// 44/168 scale of the 168 px one, so docking a per-glyph wordmark lands on the header wordmark.
import React from 'react';
import { useCurrentFrame } from 'remotion';
import { HEADER_HANDOFF, SHOTS } from '../timeline';
import { CANVAS, FONT, HEADER, PAPER, TYPE, bezier, ease, mix, prog } from '../tokens';

const p = PAPER;

// ── wordmark (frame coords) ──
const WORD = 'flowmaid';
const BIG = 168; // px, Archivo 900
const WORD_CX = 540; // frame x of the wordmark box centre during the hold
const WORD_CY = 570; // frame y of the wordmark box centre (line-height 1 box); lockup optically centred above the caption
const GLYPH_DUR = 12;
const glyphDelay = (i: number) => 10 + 3 * i; // last glyph (i = 7) lands at 43
const pressEase = bezier(0.2, 0.7, 0.25, 1); // demo letterpress easing

// Archivo 900 metrics at 168 px (measured): line box 168, baseline 140.5 below the box top,
// ascender ink top 16.5, ink bottom 143.5. The glint sits 10 px under the baseline.
const GLINT_TOP_EM = (140.5 + 10) / 168;

// dock window (local frames). Ends one frame before HEADER_HANDOFF.to so that the overlay's last
// frame (87) is already exactly in the header slot; Chrome's wordmark takes over at 88.
const DOCK_FROM = HEADER_HANDOFF.from - SHOTS.open.from; // 74
const DOCK_DUR = HEADER_HANDOFF.to - HEADER_HANDOFF.from - 1; // 13

// ── crosshair + kicker (canvas-local = frame - 96) ──
const CROSS = 64;
const CROSS_CY = 420 - CANVAS.top; // frame y 420: 50 px above the ascender ink top (frame 502.5)
const KICKER = 'OPEN-SOURCE RUST CRATE';
const KICK_START = 28;
const KICK_PER_CHAR = 0.7;
const KICK_DONE = KICK_START + KICKER.length * KICK_PER_CHAR; // 43.4
const CURSOR_STOP = 74;
const KICK_CY = 682 - CANVAS.top; // frame y 682: ~44 px under the wordmark ink (frame 629.5)
const crossVEase = bezier(0.3, 0, 0.2, 1);

/** Canvas-local layer of the opening: crosshair + typed kicker. The wordmark is OpenOverlay. */
export const OpenShot: React.FC = () => {
  const frame = useCurrentFrame();

  // crosshair: pathLength 100, vertical 0-9, horizontal 8-18 (linear), fade 24-34
  const vDraw = 100 * (1 - crossVEase(prog(frame, 0, 9)));
  const hDraw = 100 * (1 - prog(frame, 8, 10));
  const crossOp = 1 - prog(frame, 24, 10);

  // kicker typewriter + block cursor (2 f on / 2 f off after typing), gone by 78
  const chars = frame < KICK_START ? 0 : Math.min(KICKER.length, Math.floor((frame - KICK_START) / KICK_PER_CHAR));
  const cursorOn = (() => {
    if (frame < KICK_START || frame >= CURSOR_STOP) return false;
    if (frame < KICK_DONE) return true;
    return Math.floor((frame - KICK_DONE) / 2) % 2 === 0;
  })();
  const rule = ease.expo(prog(frame, KICK_START, 8));
  const kickOp = 1 - ease.quiet(prog(frame, 74, 4));

  return (
    <div style={{ position: 'absolute', inset: 0 }}>
      {crossOp > 0 ? (
        <svg
          width={CROSS}
          height={CROSS}
          viewBox={`0 0 ${CROSS} ${CROSS}`}
          style={{ position: 'absolute', left: 540 - CROSS / 2, top: CROSS_CY - CROSS / 2, opacity: crossOp, overflow: 'visible' }}
        >
          {/* x/y 32.5 keep the 3 px stroke on whole pixels */}
          <line x1={32.5} y1={0} x2={32.5} y2={CROSS} stroke={p.accent} strokeWidth={3} strokeLinecap="butt" pathLength={100} strokeDasharray={100} strokeDashoffset={vDraw} />
          <line x1={0} y1={32.5} x2={CROSS} y2={32.5} stroke={p.accent} strokeWidth={3} strokeLinecap="butt" pathLength={100} strokeDasharray={100} strokeDashoffset={hDraw} />
        </svg>
      ) : null}

      {frame >= KICK_START && kickOp > 0 ? (
        <div
          style={{
            position: 'absolute', left: 0, right: 0, top: KICK_CY - 20, height: 40, display: 'flex', alignItems: 'center', justifyContent: 'center',
            gap: 16, opacity: kickOp,
          }}
        >
          <span style={{ width: 40, height: 1, background: p.accent, transformOrigin: 'left', transform: `scaleX(${rule})` }} />
          <span
            style={{
              position: 'relative', fontFamily: FONT.mono, fontWeight: 500, fontSize: 26, lineHeight: 1, letterSpacing: TYPE.eyebrowTracking,
              color: p.accent, whiteSpace: 'pre', marginRight: `-${TYPE.eyebrowTracking}`,
            }}
          >
            {/* the full line is laid out from the start (untyped part transparent) so the centred
                kicker never shifts while it types */}
            <span style={{ position: 'relative' }}>
              {KICKER.slice(0, chars)}
              <span
                style={{
                  position: 'absolute', left: '100%', top: '50%', width: 14, height: 24, marginTop: -12,
                  background: p.accent, opacity: cursorOn ? 1 : 0,
                }}
              />
            </span>
            <span style={{ color: 'transparent' }}>{KICKER.slice(chars)}</span>
          </span>
        </div>
      ) : null}
    </div>
  );
};

/**
 * Frame-coordinate overlay above the chrome: the letterpressed wordmark, held, then docked into the
 * header slot (left edge x 56, centre y 48, 44 px). Main mounts it for abs 0..88.
 */
export const OpenOverlay: React.FC = () => {
  const frame = useCurrentFrame();
  const d = ease.inout(prog(frame, DOCK_FROM, DOCK_DUR));
  // size shrinks in log space (uniform-feeling zoom); the anchor slides from the box centre to the
  // box's left edge so the end state is "left edge at HEADER.wordmarkX".
  const size = BIG * Math.pow(HEADER.wordmarkSize / BIG, d);
  const left = mix(WORD_CX, HEADER.wordmarkX, d);
  const cy = mix(WORD_CY, HEADER.centerY, d);
  return (
    <div
      style={{
        position: 'absolute', left, top: cy, display: 'flex', alignItems: 'flex-end',
        transform: `translate(${(-(1 - d) * 50).toFixed(4)}%, -50%)`,
        fontFamily: FONT.display, fontWeight: 900, fontSize: size, lineHeight: 1, letterSpacing: TYPE.displayTracking,
        color: p.ink, whiteSpace: 'pre',
      }}
    >
      {WORD.split('').map((ch, i) => {
        const delay = glyphDelay(i);
        const t = pressEase(prog(frame, delay, GLYPH_DUR));
        const landed = delay + GLYPH_DUR;
        // glint: up over 4 f to the landing frame, down over 3 f (gone by 46, the hold is static)
        const glint = frame < landed ? prog(frame, landed - 4, 4) : 1 - prog(frame, landed, 3);
        return (
          <span
            key={i}
            style={{
              position: 'relative', display: 'inline-block', opacity: t,
              transform: t < 1 ? `scale(${(1.6 - 0.6 * t).toFixed(4)})` : undefined, transformOrigin: 'center bottom',
              filter: t < 1 ? `blur(${((1 - t) * 6).toFixed(3)}px)` : undefined,
            }}
          >
            {ch}
            {glint > 0 ? (
              <span
                style={{
                  position: 'absolute', left: '50%', top: `${GLINT_TOP_EM}em`, height: 2, width: `${(glint * 100).toFixed(2)}%`,
                  transform: 'translateX(-50%)', background: p.accent, opacity: glint,
                }}
              />
            ) : null}
          </span>
        );
      })}
    </div>
  );
};
