// Shot 2 `render` (abs 90-228, 138 f) — canvas-materialize-moves, style B "diagram-cascade"
// (references/shots/interaction/canvas-materialize-moves.md, demo
// demos/interaction/canvas-materialize-moves/DiagramCascadeBuild.tsx), re-skinned to Paper.
// Canvas-local coordinates; useCurrentFrame() is local to the shot.
//
//   2-24 / 6-28   code pane (HERO.MMD) and SVG pane (HERO.SVG, dotted canvas) draw their 1 px rules
//   6-50          hero.mmd types in line bursts (line i over 4 f from 6 + 4.4 i), accent caret
//   50            confirm: code pane rules turn accent, MERMAID tag filled
//   54            submit: FLOWMAID tag filled; accent 1 px drip line from the code pane edge (at the
//                 `A([Request])` line) to the Request node, drawn 54-62 (expo), faded 110-120
//   54-126        camera pulls back 1.25 -> 1 (ease.inout); the tree grows in the space it reveals
//   60..108       cascade of the REAL hero.svg: Request 60, Auth? 72, API Gateway 84, 401 89, Cache 96,
//                 Database 101, Response 108. Every edge draws over 16 f (dashoffset, expo) starting
//                 10 f before its target node ("lines lead nodes"); its arrowhead appears once the
//                 stroke is ~complete; yes/no chips pop when their edge completes; the Services frame
//                 draws as an outline from 80. Node pop = scale 0.6 -> 1 from the TOP-EDGE midpoint +
//                 opacity, spring(damping 20, stiffness 190) (3.6 % overshoot on the spring).
//   114-134       breath: the drawing scales 1 -> 1.02 -> 1 around its own ink centre
//   134-137       static (the last frame is held under the next wipe)
//
// Card pitfalls kept: cascade order with edges leading nodes by 10 f, 16 f edge draw, node scale
// origin = top-edge midpoint, camera pull-back simultaneous with the cascade, breath around the
// tree's visual centre (<= 1.06). Adapted (DESIGN.md 8.2 / 10): line-burst typing instead of
// 0.95-1.2 f/char, <= 4 % overshoot, no comet glow / aurora / gradient strokes / shadows.
// Deviation from DESIGN.md 8.2 (camera framing): at 1.25x the camera scales about canvas (0, 40)
// instead of centring the code pane at (540, 490). The spec framing put the pane header at canvas
// y -22 (HERO.MMD title cut by the canvas edge) and the Request / Auth? nodes off-screen right
// when they pop (60 / 72, camera still ~1.25 under ease.inout). This framing keeps the whole code
// pane (header included) in view with the code at 27.5 px, and the root node, its drip line and
// Auth? on screen for their pops.
import React from 'react';
import { spring, useCurrentFrame } from 'remotion';
import { SceneLabel, Pane, PANE_HEADER_H } from '../lib/Pane';
import { EASE, ramp } from '../lib/Polish';
import { PAPER, ease, mix, prog } from '../tokens';
import { CODE, HeroCode, lineLength } from './a/HeroCode';
import { rectPerimeter, useHeroSvg, type Box, type Hero } from './a/heroSvg';

const p = PAPER;
const FPS = 30;

// ── layout (canvas-local) ──
const CODE_PANE = { x: 56, y: 104, w: 420, h: 820 } as const;
const SVG_PANE = { x: 500, y: 104, w: 524, h: 820 } as const;
const BODY_W = SVG_PANE.w - 2; // pane body: left 1, right 1, top 48, bottom 1
const BODY_H = SVG_PANE.h - PANE_HEADER_H - 1;
const FIT = 1.24; // hero.svg 371 x 581 -> 460 x 720

// ── camera ──
const CAM_ZOOM = 1.25;
const CAM_FIX = { x: 0, y: 40 }; // fixed point of the zoom (see header note)
const PULL_FROM = 54;
const PULL_DUR = 72; // 54 -> 126

// ── typing ──
const TYPE_FROM = 6;
const LINE_STEP = 4.4;
const LINE_DUR = 4;
const CONFIRM = 50;
const SUBMIT = 54;
const lineStart = (i: number) => TYPE_FROM + LINE_STEP * i;

// ── cascade (node ids from public/hero.mmd -> node names in hero.svg) ──
const NODE_NAME: Record<string, string> = { A: 'Request', B: 'Auth?', C: 'API Gateway', D: '401', E: 'Cache', F: 'Database', G: 'Response' };
const NODE_START: Record<string, number> = { A: 60, B: 72, C: 84, D: 89, E: 96, F: 101, G: 108 };
const START_BY_NAME: Record<string, number> = Object.fromEntries(Object.entries(NODE_NAME).map(([id, n]) => [n, NODE_START[id]]));
const LEAD = 10; // edges start 10 f before their target node
const EDGE_DUR = 16;
const MARKER_AT = 0.98; // arrowhead appears once the stroke is ~complete
const CHIP_AFTER = 10; // chip pops 10 f after its edge starts (stroke ~99 %)
const FRAME_FROM = 80; // Services frame outline
const DRIP_FROM = SUBMIT;
const DRIP_DUR = 8;
const DRIP_FADE = 110;
const BREATH_FROM = 114;
const BREATH_HALF = 10;
const BREATH_AMP = 0.02;

const popSpring = (frame: number, start: number) =>
  spring({ frame: frame - start, fps: FPS, config: { damping: 20, stiffness: 190, mass: 1 } });

/** svg-units transform scaling around (ox, oy). */
const scaleAbout = (ox: number, oy: number, s: number) =>
  `translate(${ox.toFixed(3)} ${oy.toFixed(3)}) scale(${s.toFixed(4)}) translate(${(-ox).toFixed(3)} ${(-oy).toFixed(3)})`;

/** an edge starts LEAD f before its target, but never before its source node has popped
 *  (+2 f, the same gap every other edge has): F -> G would otherwise start at 98, before
 *  Database (101) exists, and draw out of empty canvas. It starts at 103 and arrives ~113;
 *  E -> G still leads Response (108). */
const edgeStartOf = (from: string, to: string) => Math.max((NODE_START[to] ?? 0) - LEAD, (NODE_START[from] ?? 0) + 2);

const HeroDrawing: React.FC<{ hero: Hero; frame: number }> = ({ hero, frame }) => {
  const edgeStart = (e: { from: string; to: string }) => edgeStartOf(e.from, e.to);
  return (
    <svg
      width={hero.w * FIT}
      height={hero.h * FIT}
      viewBox={`0 0 ${hero.w} ${hero.h}`}
      fontFamily={hero.fontFamily}
      fontSize={hero.fontSize}
      style={{ position: 'absolute', left: 0, top: 0, overflow: 'visible' }}
    >
      <defs dangerouslySetInnerHTML={{ __html: hero.defs }} />

      {/* subgraph frames: outline draws, then the frame fill and its title come in */}
      {hero.frames.map((f, i) => {
        const q = ease.expo(prog(frame, FRAME_FROM, EDGE_DUR));
        if (q <= 0) return null;
        const per = rectPerimeter(f.box, f.rx);
        return (
          <rect
            key={`f${i}`}
            x={f.box.x} y={f.box.y} width={f.box.w} height={f.box.h} rx={f.rx}
            fill={f.fill} fillOpacity={ease.quiet(prog(frame, FRAME_FROM + 6, 10))}
            stroke={f.stroke} strokeWidth={f.strokeWidth}
            strokeDasharray={`${per} ${per}`} strokeDashoffset={per * (1 - q)}
          />
        );
      })}

      {/* edges: lines lead nodes */}
      {hero.edges.map((e, i) => {
        const q = ease.expo(prog(frame, edgeStart(e), EDGE_DUR));
        if (q <= 0) return null;
        return (
          <path
            key={`e${i}`}
            d={e.d} fill="none" stroke={e.stroke} strokeWidth={e.strokeWidth}
            strokeDasharray={`${e.len} ${e.len}`} strokeDashoffset={(e.len * (1 - q)).toFixed(3)}
            markerEnd={q >= MARKER_AT && e.marker ? e.marker : undefined}
          />
        );
      })}

      {/* nodes: pop from the top-edge midpoint */}
      {hero.nodes.map((n) => {
        const start = START_BY_NAME[n.name];
        if (start === undefined || frame < start) return null;
        const s = 0.6 + 0.4 * popSpring(frame, start);
        const op = ease.quiet(prog(frame, start - 1, 6)); // already ~40 % on its start frame (no lone arrowhead)
        return (
          <g
            key={n.name}
            opacity={op}
            transform={scaleAbout(n.box.x + n.box.w / 2, n.box.y, s)}
            dangerouslySetInnerHTML={{ __html: n.html }}
          />
        );
      })}

      {/* yes / no chips pop when their edge completes */}
      {hero.chips.map((c, i) => {
        const e = hero.edges.find((x) => x.label === c.label);
        if (!e) return null;
        const start = edgeStart(e) + CHIP_AFTER;
        if (frame < start) return null;
        const s = 0.6 + 0.4 * popSpring(frame, start);
        return (
          <g
            key={`c${i}`}
            opacity={ease.quiet(prog(frame, start - 1, 5))}
            transform={scaleAbout(c.box.x + c.box.w / 2, c.box.y + c.box.h / 2, s)}
            dangerouslySetInnerHTML={{ __html: c.html }}
          />
        );
      })}

      {/* subgraph titles */}
      {hero.titles.map((c, i) => {
        const op = ease.quiet(prog(frame, FRAME_FROM + 8, 8));
        if (op <= 0) return null;
        return <g key={`t${i}`} opacity={op} dangerouslySetInnerHTML={{ __html: c.html }} />;
      })}
    </svg>
  );
};

export const RenderShot: React.FC = () => {
  const frame = useCurrentFrame();
  const hero = useHeroSvg();

  // camera: 1.25 until 54, then pull back to identity (ease.inout) over 54-126
  const cam = mix(CAM_ZOOM, 1, ease.inout(prog(frame, PULL_FROM, PULL_DUR)));

  // typing: line bursts, caret on the active line until submit
  const typed = CODE.map((_, i) => Math.floor(lineLength(i) * prog(frame, lineStart(i), LINE_DUR)));
  let caretLine = 0;
  for (let i = 0; i < CODE.length; i++) if (frame >= lineStart(i)) caretLine = i;
  const caret = frame >= TYPE_FROM && frame < SUBMIT;
  const confirmed = frame >= CONFIRM;
  const submitted = frame >= SUBMIT;

  // hero drawing placement inside the SVG pane body (centred, FIT scale)
  const svgW = (hero?.w ?? 371) * FIT;
  const svgH = (hero?.h ?? 581) * FIT;
  const ox = (BODY_W - svgW) / 2;
  const oy = (BODY_H - svgH) / 2;
  // canvas-local origin of the drawing (pane body starts at +1, +48)
  const cvX = SVG_PANE.x + 1 + ox;
  const cvY = SVG_PANE.y + PANE_HEADER_H + oy;

  // breath around the drawing's ink centre
  const br = ramp(frame, BREATH_FROM, BREATH_HALF, EASE.smooth) - ramp(frame, BREATH_FROM + BREATH_HALF, BREATH_HALF, EASE.smooth);
  const breath = 1 + BREATH_AMP * br;
  const ink: Box = hero?.ink ?? { x: 0, y: 0, w: 371, h: 581 };
  const inkCx = (ink.x + ink.w / 2) * FIT;
  const inkCy = (ink.y + ink.h / 2) * FIT;

  // drip: code pane right edge -> Request node's left end, at the node's centre height (which
  // falls inside the `A([Request]) --> B{Auth?}` code line band)
  const req = hero?.nodes.find((n) => n.name === NODE_NAME.A);
  const dripX1 = CODE_PANE.x + CODE_PANE.w;
  const dripX2 = req ? cvX + req.box.x * FIT : dripX1;
  const dripY = req ? Math.round(cvY + (req.box.y + req.box.h / 2) * FIT) : 0;
  const dripQ = ease.expo(prog(frame, DRIP_FROM, DRIP_DUR));
  const dripOp = 1 - ease.quiet(prog(frame, DRIP_FADE, 10));
  const headOp = 1 - ease.quiet(prog(frame, DRIP_FROM + DRIP_DUR - 2, 4));

  return (
    <div style={{ position: 'absolute', inset: 0 }}>
      <div
        style={{
          position: 'absolute', inset: 0, transformOrigin: `${CAM_FIX.x}px ${CAM_FIX.y}px`,
          transform: cam !== 1 ? `scale(${cam.toFixed(5)})` : undefined,
        }}
      >
        <Pane
          p={p} frame={frame} {...CODE_PANE} start={2} dur={22} title="hero.mmd" tag="mermaid" tagActive={confirmed}
          fill="panel" lineColor={confirmed ? p.accent : undefined}
        >
          <HeroCode typed={typed} caretLine={caretLine} caret={caret} />
        </Pane>
        <Pane p={p} frame={frame} {...SVG_PANE} start={6} dur={22} title="hero.svg" tag="flowmaid" tagActive={submitted} fill="canvas">
          {hero ? (
            <div
              style={{
                position: 'absolute', left: ox, top: oy, width: svgW, height: svgH, transformOrigin: `${inkCx}px ${inkCy}px`,
                transform: breath !== 1 ? `scale(${breath.toFixed(5)})` : undefined,
              }}
            >
              <HeroDrawing hero={hero} frame={frame} />
            </div>
          ) : null}
        </Pane>

        {/* drip line (accent, 1 px) + a 6 px head square while it travels */}
        {req && dripQ > 0 && dripOp > 0 ? (
          <>
            <div
              style={{
                position: 'absolute', left: dripX1, top: dripY, width: (dripX2 - dripX1) * dripQ, height: 1, background: p.accent, opacity: dripOp,
              }}
            />
            {headOp > 0 ? (
              <div
                style={{
                  position: 'absolute', left: dripX1 + (dripX2 - dripX1) * dripQ - 3, top: dripY - 2.5, width: 6, height: 6,
                  background: p.accent, opacity: headOp,
                }}
              />
            ) : null}
          </>
        ) : null}
      </div>

      <SceneLabel p={p} frame={frame} n="01" label="render" />
    </div>
  );
};

