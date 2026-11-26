// Package D private helper: the six "group photo" tiles of the outro (one representative per
// feature shot), Pane-like: 280 x 200, 36 px header with a mono 17 px label, 1 px rules, square
// corners, NO shadow. Bodies are real assets from public/ (svg renders + clip stills).
// Text inside the tile bodies is texture (clip stills at ~0.15-0.35x); the tile LABELS are the
// readable layer. Colours from tokens.PAPER only (the SVG renders / clip stills keep their own).
import React from 'react';
import { Img, staticFile } from 'remotion';
import { FONT, PAPER, SYNTAX, TYPE, dotBackground } from '../../tokens';

const p = PAPER;

export const TILE = { w: 280, h: 200, header: 36 } as const;
export const TILE_BODY = { w: TILE.w - 2, h: TILE.h - TILE.header - 1 } as const; // 278 x 163

type Crop = { x: number; y: number; w: number; h: number };

/** a source-px crop of a still, scaled to cover `w` x `h` (crop aspect ~ box aspect). */
const CropImg: React.FC<{ src: string; srcW: number; srcH: number; crop: Crop; w: number; h: number }> = ({ src, srcW, srcH, crop, w, h }) => {
  const s = Math.max(w / crop.w, h / crop.h);
  const ox = (w - crop.w * s) / 2; // centre the crop if the aspects differ slightly
  const oy = (h - crop.h * s) / 2;
  return (
    <div style={{ position: 'absolute', left: 0, top: 0, width: w, height: h, overflow: 'hidden' }}>
      <Img
        src={staticFile(src)}
        style={{ position: 'absolute', left: ox - crop.x * s, top: oy - crop.y * s, width: srcW * s, height: srcH * s, display: 'block', maxWidth: 'none' }}
      />
    </div>
  );
};

/** an SVG render contained in a box with padding. */
const SvgFit: React.FC<{ file: string; vw: number; vh: number; w: number; h: number; pad: number }> = ({ file, vw, vh, w, h, pad }) => {
  const s = Math.min((w - 2 * pad) / vw, (h - 2 * pad) / vh);
  return (
    <Img
      src={staticFile(file)}
      style={{ position: 'absolute', left: (w - vw * s) / 2, top: (h - vh * s) / 2, width: vw * s, height: vh * s, display: 'block', maxWidth: 'none' }}
    />
  );
};

const B = TILE_BODY;

// ── bodies ──

// public/hero.mmd with the playground's syntax colours (same token split as the approved fx.html).
type Tok = [keyof typeof SYNTAX | '', string];
const HERO: Tok[][] = [
  [['header', 'flowchart'], ['', ' '], ['dir', 'TD']],
  [['', '  A'], ['pu', '(['], ['text', 'Request'], ['pu', '])'], ['', ' '], ['op', '-->'], ['', ' B'], ['pu', '{'], ['text', 'Auth?'], ['pu', '}']],
  [['', '  B '], ['op', '-->'], ['pu', '|'], ['str', 'yes'], ['pu', '|'], ['', ' C'], ['pu', '['], ['text', 'API Gateway'], ['pu', ']']],
  [['', '  B '], ['op', '-->'], ['pu', '|'], ['str', 'no'], ['pu', '|'], ['', ' D'], ['pu', '['], ['text', '401'], ['pu', ']']],
  [['', '  '], ['kw', 'subgraph'], ['', ' Services']],
  [['', '    C '], ['op', '-->'], ['', ' E'], ['pu', '[('], ['text', 'Cache'], ['pu', ')]']],
  [['', '    C '], ['op', '-->'], ['', ' F'], ['pu', '[('], ['text', 'Database'], ['pu', ')]']],
  [['', '  '], ['kw', 'end']],
  [['', '  E '], ['op', '-->'], ['', ' G'], ['pu', '(['], ['text', 'Response'], ['pu', '])']],
  [['', '  F '], ['op', '-->'], ['', ' G']],
];
const CODE_W = 152;

/** 01 RENDER: text in, SVG out — hero.mmd (panel) | svg/hero.svg (371 x 581) on the playground dots. */
export const BodyRender: React.FC = () => (
  <div style={{ position: 'absolute', inset: 0 }}>
    <div style={{ position: 'absolute', left: 0, top: 0, width: CODE_W, height: B.h, background: p.panel }}>
      <div
        style={{
          position: 'absolute', left: 9, top: 11, fontFamily: FONT.mono, fontWeight: 400, fontSize: 8.4, lineHeight: '14px', whiteSpace: 'pre',
          color: p.ink, fontVariantLigatures: 'none',
        }}
      >
        {HERO.map((line, i) => (
          <div key={i}>
            {line.map(([k, t], j) => (
              <span key={j} style={k ? { color: SYNTAX[k] } : undefined}>
                {t}
              </span>
            ))}
          </div>
        ))}
      </div>
    </div>
    <div style={{ position: 'absolute', left: CODE_W, top: 0, width: 1, height: B.h, background: p.line2 }} />
    <div style={{ position: 'absolute', left: CODE_W + 1, top: 0, width: B.w - CODE_W - 1, height: B.h, ...dotBackground(p) }}>
      <SvgFit file="svg/hero.svg" vw={371} vh={581} w={B.w - CODE_W - 1} h={B.h} pad={8} />
    </div>
  </div>
);

/** 02 SPEED: the verified claim "~113x" (Archivo 900) + its comparison label + its qualifier
 *  (BRIEF: geometric mean over 25 sample diagrams; this tile is also on the cover still). */
const speedLabel: React.CSSProperties = {
  fontFamily: FONT.mono, fontWeight: 500, fontSize: 15, lineHeight: 1, letterSpacing: TYPE.labelTracking, textTransform: 'uppercase',
};
export const BodySpeed: React.FC = () => (
  <div style={{ position: 'absolute', inset: 0, background: p.panel, display: 'flex', flexDirection: 'column', justifyContent: 'center', paddingLeft: 22 }}>
    <div style={{ fontFamily: FONT.display, fontWeight: 900, fontSize: 72, lineHeight: 0.9, letterSpacing: TYPE.displayTracking, color: p.ink, whiteSpace: 'pre' }}>
      ~113<span style={{ color: p.accent }}>×</span>
    </div>
    <div style={{ ...speedLabel, marginTop: 14, color: p.dim }}>vs mermaid.js</div>
    <div style={{ ...speedLabel, marginTop: 8, color: p.muted }}>geo mean · 25 diagrams</div>
  </div>
);

/** 04 FLOWMAID-WEB: the playground canvas mid-drag (frames/web-drag.png, canvas part x > 730). */
export const BodyBrowser: React.FC = () => (
  <CropImg src="frames/web-drag.png" srcW={1590} srcH={1064} crop={{ x: 750, y: 82, w: 840, h: 493 }} w={B.w} h={B.h} />
);

/** 03 DIAGRAM TYPES: 2 x 2 mini grid of real renders (sequence, pie, mindmap, sankey). */
const MINI: { file: string; vw: number; vh: number }[] = [
  { file: 'svg/sequence.svg', vw: 718, vh: 1032 },
  { file: 'svg/pie.svg', vw: 470, vh: 334 },
  { file: 'svg/mindmap.svg', vw: 793, vh: 443 },
  { file: 'svg/sankey.svg', vw: 600, vh: 400 },
];
export const BodyTypes: React.FC = () => {
  const cw = B.w / 2;
  const ch = B.h / 2;
  return (
    <div style={{ position: 'absolute', inset: 0, background: p.canvas }}>
      {MINI.map((m, i) => (
        <div key={m.file} style={{ position: 'absolute', left: (i % 2) * cw, top: Math.floor(i / 2) * ch, width: cw, height: ch }}>
          <SvgFit file={m.file} vw={m.vw} vh={m.vh} w={cw} h={ch} pad={6} />
        </div>
      ))}
      <div style={{ position: 'absolute', left: cw, top: 0, width: 1, height: B.h, background: p.line2 }} />
      <div style={{ position: 'absolute', left: 0, top: ch, width: B.w, height: 1, background: p.line2 }} />
    </div>
  );
};

/** 05 FLOWCLI: the --watch pane mid-drag (frames/flowcli-drag.png, right pane). */
export const BodyFlowcli: React.FC = () => (
  <CropImg src="frames/flowcli-drag.png" srcW={2044} srcH={1620} crop={{ x: 960, y: 580, w: 1050, h: 616 }} w={B.w} h={B.h} />
);

/** 06 DBX: the ERD (frames/dbx-erd.png). */
export const BodyDbx: React.FC = () => (
  <CropImg src="frames/dbx-erd.png" srcW={2240} srcH={1504} crop={{ x: 100, y: 186, w: 2040, h: 1196 }} w={B.w} h={B.h} />
);

/** Tile frame. `land` 0..1 = landing pulse (border line2 -> ink). */
export const Tile: React.FC<{ n: string; label: string; land: number; children: React.ReactNode }> = ({ n, label, land, children }) => (
  <div style={{ position: 'absolute', left: 0, top: 0, width: TILE.w, height: TILE.h, background: p.bg }}>
    <div
      style={{
        position: 'absolute', left: 1, right: 1, top: 1, height: TILE.header - 1, display: 'flex', alignItems: 'center', gap: 10, padding: '0 12px',
        borderBottom: `1px solid ${p.line2}`, background: p.bg, fontFamily: FONT.mono, fontWeight: 500, fontSize: 17, lineHeight: 1,
        letterSpacing: TYPE.labelTracking, textTransform: 'uppercase',
      }}
    >
      <span style={{ color: p.accent }}>{n}</span>
      <span style={{ color: p.dim }}>{label}</span>
    </div>
    <div style={{ position: 'absolute', left: 1, top: TILE.header, width: B.w, height: B.h, overflow: 'hidden' }}>{children}</div>
    <div style={{ position: 'absolute', inset: 0, border: `1px solid ${p.line2}`, boxSizing: 'border-box' }} />
    {land > 0.002 ? <div style={{ position: 'absolute', inset: 0, border: `1px solid ${p.ink}`, boxSizing: 'border-box', opacity: land }} /> : null}
  </div>
);
