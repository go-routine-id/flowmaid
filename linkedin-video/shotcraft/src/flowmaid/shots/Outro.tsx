// OutroShot — shot 8 `outro` (abs 907-1080, 173 f, Paper). Package D.
// Card: outro-group-photo-launch (references/shots/outro/outro-group-photo-launch.md, demo
// demos/outro/outro-group-photo-launch/OutroGroupPhotoLaunch.tsx), re-skinned to flowmaid-web Paper.
//
//   0-22    revealed by Main's mode circle from the PAPER button (dbx's last frame held under it)
//   0-40    crane on the group layer: perspective(1400px) rotateX(4deg -> 0) + scale x1.06 -> x1 (ease.inout)
//   14-41   six tiles fly into their slots, cue 14 + 3i (T1, T3, T5, T2, T4, T6), 12 f each,
//           bezier(0.34, 1.4, 0.44, 1) (real overshoot, card rule y1 > 1), offsets 420-492 px from
//           outside, rotation +/-6..8deg -> 0, scale 1.12 -> 1, SpeedBlur 0.3 x px/f (cap 22) in flight;
//           landing cue+12..+21: border line2 -> ink -> line2 + 1.5 % press (replaces the contact shadow)
//   36-143  push x1 -> x1.035 (ease.inout, zero-speed start after the crane)
//   42-50   tiles step back: opacity 0.88, saturate 0.92
//   42-62.6 "flowmaid" letterpress (glyph i: 42 + 1.8i, 8 f, scale 1.6 -> 1 from the bottom, blur 6 -> 0)
//   44-62   eyebrow "—— OPEN SOURCE · GPL-3.0-OR-LATER" types
//   58-70   accent rule 2 x 240 grows; 64-72 line2 extensions shoot to the gutters, fade 72-78
//   64-80   cargo chip draws (lib Pane, fill bg)
//   70-106  "$ cargo add flowmaid" types at 2 f/char, accent block caret (solid while typing, blinks, gone at 120)
//   100-116 links rise (mask, 6 f stagger)
//   120-173 hold; 143-173 absolutely static (R1). Fixer round 1: 7 f shorter (they went to shot 3).
//
// Group-layer scale is normalised so the END state (143+) is exactly the designed layout: the crane
// and push keep the card's relative moves (x1.06 -> x1, then +3.5 %), divided by 1.035. Without it the
// pushed tiles would end 17 px outside SAFE (x 39 / canvas y 22); with it every tile lands in SAFE.
//
// Deviations (DESIGN.md section 10 + brief): no stage light, gold dust, light band, contact shadows,
// grain or vignette; tiles settle at 0deg (grid-true); the VO8 captions are shown (drawn by Main).
// The energy peak is carried by structure, crane and fly-in only.
import React from 'react';
import { useCurrentFrame } from 'remotion';
import { FONT, PAPER, TYPE, bezier, ease, prog } from '../tokens';
import { Pane } from '../lib/Pane';
import { SpeedBlur, velocity } from '../lib/Polish';
import { MaskRise } from './d/MaskRise';
import { BodyBrowser, BodyDbx, BodyFlowcli, BodyRender, BodySpeed, BodyTypes, TILE, Tile } from './d/OutroTiles';

const p = PAPER;

// ── tiles (canvas-local slots = final layout); listed in cue order = render order ──
type TileDef = { key: string; n: string; label: string; x: number; y: number; dx: number; dy: number; rot: number; Body: React.FC };
// Fixer round 1: slots read 01 02 03 / 04 05 06 left-to-right, top-to-bottom, and the labels are
// the in-film scene labels (03 DIAGRAM TYPES, 04 FLOWMAID-WEB); the fly-in motion per slot is unchanged.
const TILES: TileDef[] = [
  { key: 't1', n: '01', label: 'Render', x: 56, y: 40, dx: -440, dy: -220, rot: -8, Body: BodyRender },
  { key: 't3', n: '03', label: 'Diagram types', x: 744, y: 40, dx: 450, dy: -200, rot: 8, Body: BodyTypes },
  { key: 't5', n: '05', label: 'Flowcli', x: 400, y: 700, dx: 0, dy: 440, rot: -6, Body: BodyFlowcli },
  { key: 't2', n: '02', label: 'Speed', x: 400, y: 40, dx: 0, dy: -420, rot: 6, Body: BodySpeed },
  { key: 't4', n: '04', label: 'flowmaid-web', x: 56, y: 700, dx: -440, dy: 220, rot: 8, Body: BodyBrowser },
  { key: 't6', n: '06', label: 'Dbx', x: 744, y: 700, dx: 420, dy: 240, rot: -8, Body: BodyDbx },
];
const CUE0 = 14;
const CUE_STEP = 3;
const FLY = 12;
const flyEase = bezier(0.34, 1.4, 0.44, 1);

// ── lockup (canvas-local) ──
const EYEBROW = 'OPEN SOURCE · GPL-3.0-OR-LATER';
const EYEBROW_CY = 284;
const WORD = 'flowmaid';
const WORD_SIZE = 150;
const WORD_TOP = 314; // line box top (line-height 1)
const pressEase = bezier(0.2, 0.7, 0.25, 1); // shot 1's letterpress easing
const RULE_Y = 462;
const RULE_W = 240;
const CHIP = { x: 250, y: 488, w: 580, h: 72 } as const;
const CMD_PROMPT = '$ ';
const CMD = 'cargo add flowmaid';
const TYPE_FROM = 70;
const TYPE_PER = 2; // frames per char -> done at 106
const CARET_STOP = 120;
const LINK1 = 'opensource.go-routine.com/flowmaid-web';
const LINK2 = 'github.com/go-routine-id/flowmaid';
const LINK1_CY = 600;
const LINK2_CY = 645;
const LINK_SIZE = 32;

const camAt = (frame: number) => {
  const crane = ease.inout(prog(frame, 0, 40));
  const push = ease.inout(prog(frame, 36, 107)); // 36 -> 143
  return { scale: (1.06 - 0.06 * crane + 0.035 * push) / 1.035, tilt: 4 * (1 - crane) };
};

export const OutroShot: React.FC = () => {
  const frame = useCurrentFrame();
  const cam = camAt(frame);
  const recede = prog(frame, 42, 8);

  // group-layer transform; identity-free when settled so text rasterises crisply
  const tf: string[] = [];
  if (cam.tilt > 1e-4) tf.push(`perspective(1400px) rotateX(${cam.tilt.toFixed(4)}deg)`);
  if (Math.abs(cam.scale - 1) > 1e-6) tf.push(`scale(${cam.scale.toFixed(6)})`);

  // eyebrow typing 44-62
  const ebChars = Math.max(0, Math.min(EYEBROW.length, Math.floor(((frame - 44) / 18) * EYEBROW.length)));
  const ebRule = ease.expo(prog(frame, 44, 10));

  // rule + extensions
  const rule = ease.expo(prog(frame, 58, 12));
  const ext = ease.expo(prog(frame, 64, 8));
  const extFade = 1 - prog(frame, 72, 6);

  // typing
  const typed = Math.max(0, Math.min(CMD.length, Math.floor((frame - TYPE_FROM) / TYPE_PER)));
  const typingDone = TYPE_FROM + CMD.length * TYPE_PER; // 106
  const caretOn = frame >= TYPE_FROM && frame < CARET_STOP && (frame < typingDone || Math.floor((frame - typingDone) / 4) % 2 === 0);

  // links
  const l1 = ease.expo(prog(frame, 100, 10));
  const l2 = ease.expo(prog(frame, 106, 10));

  return (
    <div style={{ position: 'absolute', inset: 0 }}>
      <div style={{ position: 'absolute', inset: 0, transformOrigin: '540px 540px', transform: tf.length ? tf.join(' ') : undefined }}>
        {/* ── group photo: tiles fly into their slots ── */}
        {TILES.map((t, i) => {
          const cue = CUE0 + CUE_STEP * i;
          if (frame < cue) return null;
          const tAt = (f: number) => flyEase(prog(f, cue, FLY));
          const k = tAt(frame);
          const x = t.dx * (1 - k);
          const y = t.dy * (1 - k);
          const vx = velocity((f) => t.dx * (1 - tAt(f)), frame);
          const vy = velocity((f) => t.dy * (1 - tAt(f)), frame);
          const inFlight = frame < cue + FLY;
          const land = frame < cue + 12 ? 0 : frame < cue + 14 ? prog(frame, cue + 12, 2) : 1 - prog(frame, cue + 14, 7);
          const scale = (1.12 - 0.12 * k) * (1 - 0.015 * land);
          const rot = t.rot * (1 - k);
          const opacity = prog(frame, cue, 3) * (1 - 0.12 * recede);
          const sat = 1 - 0.08 * recede;
          const settled = !inFlight && land <= 0.002;
          const tile = (
            <div
              style={{
                position: 'absolute', left: t.x, top: t.y, width: TILE.w, height: TILE.h, opacity,
                transform: settled ? undefined : `translate(${x.toFixed(3)}px, ${y.toFixed(3)}px) rotate(${rot.toFixed(4)}deg) scale(${scale.toFixed(5)})`,
                transformOrigin: 'center center', filter: sat < 0.9999 ? `saturate(${sat.toFixed(4)})` : undefined,
              }}
            >
              <Tile n={t.n} label={t.label} land={land}>
                <t.Body />
              </Tile>
            </div>
          );
          return inFlight ? (
            <SpeedBlur key={t.key} vx={vx} vy={vy} amount={0.3} max={22}>
              {tile}
            </SpeedBlur>
          ) : (
            <React.Fragment key={t.key}>{tile}</React.Fragment>
          );
        })}

        {/* ── lockup ── */}
        {/* eyebrow: 40 px accent rule + typed label (laid out in full so the centred line never shifts) */}
        {frame >= 44 ? (
          <div style={{ position: 'absolute', left: 0, right: 0, top: EYEBROW_CY - 14, height: 28, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 16 }}>
            <span style={{ width: 40, height: 1, background: p.accent, transformOrigin: 'left', transform: `scaleX(${ebRule})` }} />
            <span
              style={{
                fontFamily: FONT.mono, fontWeight: 500, fontSize: 22, lineHeight: 1, letterSpacing: TYPE.eyebrowTracking, color: p.accent,
                whiteSpace: 'pre', marginRight: `-${TYPE.eyebrowTracking}`,
              }}
            >
              {EYEBROW.slice(0, ebChars)}
              <span style={{ color: 'transparent' }}>{EYEBROW.slice(ebChars)}</span>
            </span>
          </div>
        ) : null}

        {/* wordmark letterpress */}
        <div
          style={{
            position: 'absolute', left: 0, right: 0, top: WORD_TOP, height: WORD_SIZE, display: 'flex', justifyContent: 'center', alignItems: 'flex-end',
          }}
        >
          <div
            style={{
              display: 'flex', alignItems: 'flex-end', paddingRight: TYPE.displayTracking.replace('-', ''), fontFamily: FONT.display, fontWeight: 900,
              fontSize: WORD_SIZE, lineHeight: 1, letterSpacing: TYPE.displayTracking, color: p.ink, whiteSpace: 'pre',
            }}
          >
            {WORD.split('').map((ch, i) => {
              const t = pressEase(prog(frame, 42 + 1.8 * i, 8));
              return (
                <span
                  key={i}
                  style={{
                    display: 'inline-block', opacity: t, transformOrigin: 'center bottom',
                    transform: t < 1 ? `scale(${(1.6 - 0.6 * t).toFixed(4)})` : undefined,
                    filter: t < 1 ? `blur(${((1 - t) * 6).toFixed(3)}px)` : undefined,
                  }}
                >
                  {ch}
                </span>
              );
            })}
          </div>
        </div>

        {/* accent rule + gutter extensions */}
        <div style={{ position: 'absolute', left: 540 - RULE_W / 2, top: RULE_Y, width: RULE_W, height: 2, background: p.accent, transform: `scaleX(${rule})` }} />
        {ext > 0 && extFade > 0 ? (
          <>
            <div
              style={{
                position: 'absolute', left: 56, top: RULE_Y, width: 540 - RULE_W / 2 - 56, height: 1, background: p.line2, opacity: extFade,
                transformOrigin: 'right', transform: `scaleX(${ext})`,
              }}
            />
            <div
              style={{
                position: 'absolute', left: 540 + RULE_W / 2, top: RULE_Y, width: 1024 - (540 + RULE_W / 2), height: 1, background: p.line2, opacity: extFade,
                transformOrigin: 'left', transform: `scaleX(${ext})`,
              }}
            />
          </>
        ) : null}

        {/* cargo chip */}
        <Pane p={p} frame={frame} x={CHIP.x} y={CHIP.y} w={CHIP.w} h={CHIP.h} start={64} dur={16} fill="bg">
          {frame >= TYPE_FROM ? (
            <div
              style={{
                position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', fontFamily: FONT.mono, fontWeight: 400,
                fontSize: 40, lineHeight: 1, color: p.ink, whiteSpace: 'pre', fontVariantLigatures: 'none',
              }}
            >
              <span style={{ position: 'relative' }}>
                <span style={{ color: p.accent }}>{CMD_PROMPT}</span>
                {CMD.slice(0, typed)}
                <span
                  style={{
                    position: 'absolute', top: '50%', marginTop: -21, width: 22, height: 42, background: p.accent, opacity: caretOn ? 1 : 0,
                    // the caret sits right after the last typed character (mono: 0.6 em advance)
                    left: `${((CMD_PROMPT.length + typed) * 0.6).toFixed(3)}em`, marginLeft: 2,
                  }}
                />
                <span style={{ color: 'transparent' }}>{CMD.slice(typed)}</span>
              </span>
            </div>
          ) : null}
        </Pane>

        {/* links */}
        <div style={{ position: 'absolute', left: 0, right: 0, top: LINK1_CY - 20, display: 'flex', justifyContent: 'center' }}>
          <MaskRise t={l1} height={40}>
            <span style={{ fontFamily: FONT.mono, fontWeight: 400, fontSize: LINK_SIZE, lineHeight: 1.25, color: p.accent, whiteSpace: 'pre' }}>{LINK1}</span>
          </MaskRise>
        </div>
        <div style={{ position: 'absolute', left: 0, right: 0, top: LINK2_CY - 20, display: 'flex', justifyContent: 'center' }}>
          <MaskRise t={l2} height={40}>
            <span style={{ fontFamily: FONT.mono, fontWeight: 400, fontSize: LINK_SIZE, lineHeight: 1.25, color: p.dim, whiteSpace: 'pre' }}>{LINK2}</span>
          </MaskRise>
        </div>
      </div>
    </div>
  );
};

// exported for QA/notes: group scale at a local frame (1 from 143 on)
export const outroCamAt = camAt;
export const OUTRO_TILE_CUES = TILES.map((t, i) => ({ key: t.key, cue: CUE0 + CUE_STEP * i, lands: CUE0 + CUE_STEP * i + FLY }));
