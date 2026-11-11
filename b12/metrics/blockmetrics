#!/usr/bin/env node
'use strict';
// blockmetrics - layout metrics for flowmaid package B12 "subgraphs as blocks".
//
// Reads SVGs drawn by flowmaid (scene::to_svg) or by mermaid (flowchart-v2 /
// stateDiagram-v2 renderers) and prints one TSV row per diagram. Node.js
// standard library only. See README.md next to this file for the columns
// and for how every number was validated.
//
//   blockmetrics [options] file.svg ...
//     --struct FILE        structure JSON from structdump (one input file)
//     --struct-dir DIR     look up DIR/<name>.struct.json per input
//     --dir TD|LR|RL|BT    flow direction when there is no structure file
//     --engine fm|mermaid  default: detected from the SVG
//     --label TEXT         value of the `engine` column (default fm / mermaid)
//     --detail             per-frame / per-edge / per-label tables on stderr
//     --overlay DIR        write <name>.<engine>.overlay.svg with the findings drawn in
//     --check-scene        flowmaid only: compare the SVG's geometry with struct.scene
//     --json               print the model + metrics as JSON instead of TSV
//     --no-header          do not print the TSV header
//     --selftest           run the hand-checked cases in ./selftest and exit
//
// Lengths are px in the SVG's user space. "Flow axis" = y for TD/BT, x for LR/RL.

const fs = require('fs');
const path = require('path');

// ---------------------------------------------------------------- tolerances
const EPS_CONTACT = 0.5;   // two strokes closer than this are "in contact"
const DELTA_SIDE = 2.0;    // how far before/after a contact the side is read
const FRAME_INSET = 0.5;   // a run counts as inside a frame this far in from its border
const RUN_MIN = 2.0;       // shorter runs inside a foreign frame / node / chip are grazes
const STRAIGHT_ANG = 1.5;  // degrees a segment may turn and still continue a straight run
const STRAIGHT_OFF = 0.75; // px a point may leave the run's line
const BEND_RUN_MIN = 8.0;  // straight runs at least this long count for `bends`
const HOP_SPAN = 30.0;     // longest detour (px along the path) bridged as a crossing hop: a half circle of radius HOP_DEV and its sampling (the level top of a bridge is not counted)
const HOP_DEV = 9.0;       // farthest such a detour leaves its line (flowmaid's HOP_R_MAX)
const HOP_QUARTER_TOL = 1.0; // a bridge leaves and rejoins its line on quarter circles: as far along the line as off it, within this
const DETOUR_MIN_LEN = 200.0; // `detour_max` reads edges at least this long
const OVERLAP_MIN = 0.5;   // boxes overlap when they share more than this on both axes
const SIDE_MIN = 1.0;      // flow projections overlap when they share more than this
const LANE_RUN_MIN = 40.0; // parallel lanes: straight runs this long, overlapping this much
const LANE_PITCH = 26.0;   // neighbours no farther apart than this form one bundle
const LBL_NEAR = 12.0;     // a foreign stroke nearer than this to a chip crowds the label (flowmaid's CLEAR_GAP)
const END_NODE_TOL = 14.0; // geometric end resolution: path end to node box
const END_FRAME_TOL = 6.0; // geometric end resolution: path end to frame border

// ------------------------------------------------------------------ XML
function decodeEntities(s) {
  if (s.indexOf('&') < 0) return s;
  return s.replace(/&(#x[0-9a-fA-F]+|#[0-9]+|[a-zA-Z]+);/g, (m, e) => {
    if (e[0] === '#') {
      const code = e[1] === 'x' || e[1] === 'X' ? parseInt(e.slice(2), 16) : parseInt(e.slice(1), 10);
      return Number.isFinite(code) ? String.fromCodePoint(code) : m;
    }
    return ({ amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", nbsp: ' ' })[e] ?? m;
  });
}

// A small, forgiving XML reader: enough for the two SVG dialects. The inside
// of <style> and <foreignObject> is kept as raw text (mermaid puts HTML there).
function parseXml(src) {
  const root = { name: '#root', attrs: {}, children: [], text: '', parent: null };
  let cur = root;
  let i = 0;
  const n = src.length;
  const attrRe = /([^\s=\/<>]+)\s*=\s*(?:"([^"]*)"|'([^']*)')/g;
  while (i < n) {
    const lt = src.indexOf('<', i);
    if (lt < 0) { cur.text += decodeEntities(src.slice(i)); break; }
    if (lt > i) cur.text += decodeEntities(src.slice(i, lt));
    if (src.startsWith('<!--', lt)) { const e = src.indexOf('-->', lt + 4); i = e < 0 ? n : e + 3; continue; }
    if (src.startsWith('<![CDATA[', lt)) {
      const e = src.indexOf(']]>', lt + 9);
      cur.text += src.slice(lt + 9, e < 0 ? n : e); i = e < 0 ? n : e + 3; continue;
    }
    if (src[lt + 1] === '?' || src[lt + 1] === '!') { const e = src.indexOf('>', lt); i = e < 0 ? n : e + 1; continue; }
    // find the end of the tag, skipping quoted attribute values
    let j = lt + 1, q = null;
    while (j < n) {
      const c = src[j];
      if (q) { if (c === q) q = null; } else if (c === '"' || c === "'") q = c; else if (c === '>') break;
      j++;
    }
    const raw = src.slice(lt + 1, j);
    i = j + 1;
    if (raw[0] === '/') {
      const name = raw.slice(1).trim();
      let p = cur;
      while (p && p.name !== name) p = p.parent;
      if (p && p.parent) cur = p.parent;
      continue;
    }
    const selfClose = raw.endsWith('/');
    const body = selfClose ? raw.slice(0, -1) : raw;
    const m = /^([^\s\/]+)/.exec(body);
    if (!m) continue;
    const el = { name: m[1], attrs: {}, children: [], text: '', parent: cur };
    attrRe.lastIndex = m[1].length;
    let a;
    while ((a = attrRe.exec(body))) el.attrs[a[1]] = decodeEntities(a[2] !== undefined ? a[2] : a[3]);
    cur.children.push(el);
    if (selfClose) continue;
    if (el.name === 'style' || el.name === 'foreignObject' || el.name === 'script') {
      const close = '</' + el.name + '>';
      const e = src.indexOf(close, i);
      const inner = src.slice(i, e < 0 ? n : e);
      el.text = el.name === 'foreignObject'
        ? decodeEntities(inner.replace(/<br\s*\/?>/g, '\n').replace(/<[^>]*>/g, '')).trim()
        : '';
      i = e < 0 ? n : e + close.length;
      continue;
    }
    cur = el;
  }
  return root;
}

function textOf(el) {
  let s = el.text || '';
  for (const c of el.children) if (c.name !== 'title') s += (s && c.name === 'tspan' ? '\n' : '') + textOf(c);
  return s;
}
function num(v, d = 0) { const x = parseFloat(v); return Number.isFinite(x) ? x : d; }
function classes(el) { return (el.attrs.class || '').split(/\s+/).filter(Boolean); }
function hasClass(el, c) { return classes(el).includes(c); }

// translate(...) / matrix(...) / scale(...) of one element, as [a b c d e f]
function parseTransform(t) {
  let m = [1, 0, 0, 1, 0, 0];
  if (!t) return m;
  const re = /([a-zA-Z]+)\s*\(([^)]*)\)/g;
  let x;
  while ((x = re.exec(t))) {
    const v = x[2].split(/[\s,]+/).filter(Boolean).map(Number);
    let k;
    if (x[1] === 'translate') k = [1, 0, 0, 1, v[0] || 0, v[1] || 0];
    else if (x[1] === 'scale') k = [v[0] ?? 1, 0, 0, v[1] ?? v[0] ?? 1, 0, 0];
    else if (x[1] === 'matrix' && v.length === 6) k = v;
    else if (x[1] === 'rotate') {
      const a = (v[0] || 0) * Math.PI / 180, c = Math.cos(a), s = Math.sin(a);
      k = [c, s, -s, c, 0, 0];
      if (v.length >= 3) { // rotate about (cx, cy)
        const cx = v[1], cy = v[2];
        k = [c, s, -s, c, cx - c * cx + s * cy, cy - s * cx - c * cy];
      }
    } else continue;
    m = mul(m, k);
  }
  return m;
}
function mul(m, k) {
  return [
    m[0] * k[0] + m[2] * k[1], m[1] * k[0] + m[3] * k[1],
    m[0] * k[2] + m[2] * k[3], m[1] * k[2] + m[3] * k[3],
    m[0] * k[4] + m[2] * k[5] + m[4], m[1] * k[4] + m[3] * k[5] + m[5],
  ];
}
function apply(m, p) { return [m[0] * p[0] + m[2] * p[1] + m[4], m[1] * p[0] + m[3] * p[1] + m[5]]; }

// ------------------------------------------------------------------ paths
// SVG path data -> list of polylines (one per subpath). Curves are sampled;
// `arcs` collects every elliptical arc as {r, from, to} for the caller.
function flattenPath(d, arcsOut) {
  const tok = d.match(/[a-zA-Z]|[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g) || [];
  const subs = [];
  let cur = null, x = 0, y = 0, sx = 0, sy = 0, i = 0, cmd = '', px = 0, py = 0, prevCmd = '';
  const nx = () => parseFloat(tok[i++]);
  const isNum = () => i < tok.length && !/^[a-zA-Z]$/.test(tok[i]);
  const lineTo = (X, Y) => { if (!cur) { cur = [[x, y]]; subs.push(cur); } cur.push([X, Y]); x = X; y = Y; };
  const sample = (f, approxLen) => {
    const steps = Math.max(6, Math.min(160, Math.ceil(approxLen)));
    for (let k = 1; k <= steps; k++) { const p = f(k / steps); if (!cur) { cur = [[x, y]]; subs.push(cur); } cur.push(p); }
  };
  while (i < tok.length) {
    if (/^[a-zA-Z]$/.test(tok[i])) cmd = tok[i++];
    else if (cmd === 'M') cmd = 'L'; else if (cmd === 'm') cmd = 'l';
    const rel = cmd === cmd.toLowerCase();
    const C = cmd.toUpperCase();
    if (C === 'Z') { if (cur) { cur.push([sx, sy]); } x = sx; y = sy; cur = null; prevCmd = 'Z'; continue; }
    if (!isNum()) { prevCmd = C; continue; }
    if (C === 'M') {
      let X = nx(), Y = nx(); if (rel) { X += x; Y += y; }
      x = X; y = Y; sx = X; sy = Y; cur = [[x, y]]; subs.push(cur);
    } else if (C === 'L') {
      let X = nx(), Y = nx(); if (rel) { X += x; Y += y; } lineTo(X, Y);
    } else if (C === 'H') { let X = nx(); if (rel) X += x; lineTo(X, y);
    } else if (C === 'V') { let Y = nx(); if (rel) Y += y; lineTo(x, Y);
    } else if (C === 'C' || C === 'S') {
      let x1, y1;
      if (C === 'C') { x1 = nx(); y1 = nx(); if (rel) { x1 += x; y1 += y; } }
      else if (prevCmd === 'C' || prevCmd === 'S') { x1 = 2 * x - px; y1 = 2 * y - py; } else { x1 = x; y1 = y; }
      let x2 = nx(), y2 = nx(), X = nx(), Y = nx();
      if (rel) { x2 += x; y2 += y; X += x; Y += y; }
      const x0 = x, y0 = y;
      const approx = Math.hypot(x1 - x0, y1 - y0) + Math.hypot(x2 - x1, y2 - y1) + Math.hypot(X - x2, Y - y2);
      sample(t => {
        const u = 1 - t;
        return [u * u * u * x0 + 3 * u * u * t * x1 + 3 * u * t * t * x2 + t * t * t * X,
                u * u * u * y0 + 3 * u * u * t * y1 + 3 * u * t * t * y2 + t * t * t * Y];
      }, approx);
      px = x2; py = y2; x = X; y = Y;
    } else if (C === 'Q' || C === 'T') {
      let x1, y1;
      if (C === 'Q') { x1 = nx(); y1 = nx(); if (rel) { x1 += x; y1 += y; } }
      else if (prevCmd === 'Q' || prevCmd === 'T') { x1 = 2 * x - px; y1 = 2 * y - py; } else { x1 = x; y1 = y; }
      let X = nx(), Y = nx(); if (rel) { X += x; Y += y; }
      const x0 = x, y0 = y;
      const approx = Math.hypot(x1 - x0, y1 - y0) + Math.hypot(X - x1, Y - y1);
      sample(t => {
        const u = 1 - t;
        return [u * u * x0 + 2 * u * t * x1 + t * t * X, u * u * y0 + 2 * u * t * y1 + t * t * Y];
      }, approx);
      px = x1; py = y1; x = X; y = Y;
    } else if (C === 'A') {
      let rx = Math.abs(nx()), ry = Math.abs(nx());
      const rot = nx() * Math.PI / 180, large = nx() !== 0, sweep = nx() !== 0;
      let X = nx(), Y = nx(); if (rel) { X += x; Y += y; }
      if (arcsOut) arcsOut.push({ r: Math.max(rx, ry), from: [x, y], to: [X, Y] });
      const x0 = x, y0 = y;
      if (rx === 0 || ry === 0 || (x0 === X && y0 === Y)) { lineTo(X, Y); }
      else {
        // endpoint -> centre parameterisation (SVG 1.1 F.6.5)
        const cr = Math.cos(rot), sr = Math.sin(rot);
        const dx = (x0 - X) / 2, dy = (y0 - Y) / 2;
        const xp = cr * dx + sr * dy, yp = -sr * dx + cr * dy;
        const lam = (xp * xp) / (rx * rx) + (yp * yp) / (ry * ry);
        if (lam > 1) { const s = Math.sqrt(lam); rx *= s; ry *= s; }
        const numr = rx * rx * ry * ry - rx * rx * yp * yp - ry * ry * xp * xp;
        const den = rx * rx * yp * yp + ry * ry * xp * xp;
        let co = den === 0 ? 0 : Math.sqrt(Math.max(0, numr / den));
        if (large === sweep) co = -co;
        const cxp = co * rx * yp / ry, cyp = -co * ry * xp / rx;
        const cx = cr * cxp - sr * cyp + (x0 + X) / 2, cy = sr * cxp + cr * cyp + (y0 + Y) / 2;
        const ang = (ux, uy, vx, vy) => {
          const dot = ux * vx + uy * vy, len = Math.hypot(ux, uy) * Math.hypot(vx, vy);
          let a = Math.acos(Math.max(-1, Math.min(1, dot / len)));
          if (ux * vy - uy * vx < 0) a = -a;
          return a;
        };
        const th1 = ang(1, 0, (xp - cxp) / rx, (yp - cyp) / ry);
        let dth = ang((xp - cxp) / rx, (yp - cyp) / ry, (-xp - cxp) / rx, (-yp - cyp) / ry);
        if (!sweep && dth > 0) dth -= 2 * Math.PI; else if (sweep && dth < 0) dth += 2 * Math.PI;
        sample(t => {
          const a = th1 + dth * t, ex = rx * Math.cos(a), ey = ry * Math.sin(a);
          return [cr * ex - sr * ey + cx, sr * ex + cr * ey + cy];
        }, Math.abs(dth) * Math.max(rx, ry));
        x = X; y = Y;
      }
    } else { i++; }
    prevCmd = C;
  }
  return subs;
}

// ------------------------------------------------------------------ geometry
function bboxOfPoints(pts) {
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  for (const p of pts) { if (p[0] < x0) x0 = p[0]; if (p[0] > x1) x1 = p[0]; if (p[1] < y0) y0 = p[1]; if (p[1] > y1) y1 = p[1]; }
  return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
}
function boxUnion(a, b) {
  if (!a) return b; if (!b) return a;
  const x = Math.min(a.x, b.x), y = Math.min(a.y, b.y);
  return { x, y, w: Math.max(a.x + a.w, b.x + b.w) - x, h: Math.max(a.y + a.h, b.y + b.h) - y };
}
function boxContainsPoint(b, p, pad = 0) {
  return p[0] >= b.x - pad && p[0] <= b.x + b.w + pad && p[1] >= b.y - pad && p[1] <= b.y + b.h + pad;
}
function boxContainsBox(outer, inner, tol = 0.5) {
  return inner.x >= outer.x - tol && inner.y >= outer.y - tol &&
    inner.x + inner.w <= outer.x + outer.w + tol && inner.y + inner.h <= outer.y + outer.h + tol;
}
function boxOverlap(a, b) { // shared extent on each axis (negative = apart)
  return [Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x), Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y)];
}
function distPointBox(p, b) {
  const dx = Math.max(b.x - p[0], 0, p[0] - (b.x + b.w)), dy = Math.max(b.y - p[1], 0, p[1] - (b.y + b.h));
  return Math.hypot(dx, dy);
}
function distPointBoxBorder(p, b) {
  if (!boxContainsPoint(b, p)) return distPointBox(p, b);
  return Math.min(p[0] - b.x, b.x + b.w - p[0], p[1] - b.y, b.y + b.h - p[1]);
}
function center(b) { return [b.x + b.w / 2, b.y + b.h / 2]; }

// polyline(s) -> segment list with cumulative arclength; drops zero-length
// segments and merges collinear neighbours (so a straight run is one segment)
function toSegs(polys) {
  const segs = [];
  let s = 0;
  for (const pts of polys) {
    let run = null; // [ax, ay, bx, by]
    const flush = () => {
      if (!run) return;
      const len = Math.hypot(run[2] - run[0], run[3] - run[1]);
      if (len > 1e-9) { segs.push({ ax: run[0], ay: run[1], bx: run[2], by: run[3], len, s0: s }); s += len; }
      run = null;
    };
    for (let k = 1; k < pts.length; k++) {
      const a = pts[k - 1], b = pts[k];
      const len = Math.hypot(b[0] - a[0], b[1] - a[1]);
      if (len < 1e-9) continue;
      if (run) {
        const rl = Math.hypot(run[2] - run[0], run[3] - run[1]);
        const ux = (run[2] - run[0]) / rl, uy = (run[3] - run[1]) / rl;
        const cross = Math.abs((b[0] - run[0]) * uy - (b[1] - run[1]) * ux);
        const dot = ((b[0] - a[0]) * ux + (b[1] - a[1]) * uy) / len;
        if (cross < 1e-6 * (1 + rl + len) && dot > 0.999999) { run[2] = b[0]; run[3] = b[1]; continue; }
        flush();
      }
      run = [a[0], a[1], b[0], b[1]];
    }
    flush();
  }
  return segs;
}
function segsLength(segs) { return segs.length ? segs[segs.length - 1].s0 + segs[segs.length - 1].len : 0; }
function pointAt(segs, s) {
  if (!segs.length) return [NaN, NaN];
  if (s <= 0) return [segs[0].ax, segs[0].ay];
  for (const g of segs) {
    if (s <= g.s0 + g.len) { const t = (s - g.s0) / g.len; return [g.ax + (g.bx - g.ax) * t, g.ay + (g.by - g.ay) * t]; }
  }
  const g = segs[segs.length - 1];
  return [g.bx, g.by];
}
function closestOnSeg(g, p) {
  const dx = g.bx - g.ax, dy = g.by - g.ay;
  let t = ((p[0] - g.ax) * dx + (p[1] - g.ay) * dy) / (g.len * g.len);
  t = Math.max(0, Math.min(1, t));
  const qx = g.ax + dx * t, qy = g.ay + dy * t;
  return { t, d: Math.hypot(p[0] - qx, p[1] - qy), q: [qx, qy] };
}
function closestOnSegs(segs, p) {
  let best = null;
  for (let k = 0; k < segs.length; k++) {
    const c = closestOnSeg(segs[k], p);
    if (!best || c.d < best.d) best = { k, t: c.t, d: c.d, q: c.q, s: segs[k].s0 + c.t * segs[k].len };
  }
  return best;
}
function segsBBox(segs) {
  const pts = [];
  for (const g of segs) { pts.push([g.ax, g.ay], [g.bx, g.by]); }
  return bboxOfPoints(pts);
}
// arclength intervals of `segs` inside box b (shrunk by inset), merged
function insideIntervals(segs, b, inset) {
  const x0 = b.x + inset, y0 = b.y + inset, x1 = b.x + b.w - inset, y1 = b.y + b.h - inset;
  const out = [];
  if (x1 <= x0 || y1 <= y0) return out;
  for (const g of segs) {
    let t0 = 0, t1 = 1;
    const dx = g.bx - g.ax, dy = g.by - g.ay;
    let ok = true;
    for (const [p, q] of [[-dx, g.ax - x0], [dx, x1 - g.ax], [-dy, g.ay - y0], [dy, y1 - g.ay]]) {
      if (p === 0) { if (q < 0) { ok = false; break; } }
      else { const r = q / p; if (p < 0) { if (r > t1) { ok = false; break; } if (r > t0) t0 = r; } else { if (r < t0) { ok = false; break; } if (r < t1) t1 = r; } }
    }
    if (!ok || t1 <= t0) continue;
    out.push([g.s0 + t0 * g.len, g.s0 + t1 * g.len]);
  }
  return mergeIntervals(out, 1e-6);
}
function mergeIntervals(iv, gap) {
  iv.sort((a, b) => a[0] - b[0]);
  const out = [];
  for (const v of iv) {
    if (out.length && v[0] <= out[out.length - 1][1] + gap) out[out.length - 1][1] = Math.max(out[out.length - 1][1], v[1]);
    else out.push([v[0], v[1]]);
  }
  return out;
}

// where a segment list passes a box's border between its inside and its
// outside: [{p, side}], side = L / R / T / B (the screen's). A crossing
// exactly through a corner goes to the side the segment runs across.
function borderCrossings(segs, b) {
  const x0 = b.x, y0 = b.y, x1 = b.x + b.w, y1 = b.y + b.h;
  const out = [];
  if (x1 <= x0 || y1 <= y0) return out;
  const sideAt = (p, dx, dy) => {
    const d = [['L', Math.abs(p[0] - x0)], ['R', Math.abs(p[0] - x1)], ['T', Math.abs(p[1] - y0)], ['B', Math.abs(p[1] - y1)]];
    d.sort((u, v) => u[1] - v[1]);
    if (d[1][1] - d[0][1] > 1e-6) return d[0][0];
    // a corner: the side the segment crosses more squarely
    const horiz = Math.abs(dx) >= Math.abs(dy);
    const pick = d.slice(0, 2).map(x => x[0]);
    return pick.find(s => (horiz ? s === 'L' || s === 'R' : s === 'T' || s === 'B')) || pick[0];
  };
  for (const g of segs) {
    let t0 = 0, t1 = 1;
    const dx = g.bx - g.ax, dy = g.by - g.ay;
    let ok = true;
    for (const [p, q] of [[-dx, g.ax - x0], [dx, x1 - g.ax], [-dy, g.ay - y0], [dy, y1 - g.ay]]) {
      if (p === 0) { if (q <= 0) { ok = false; break; } }
      else { const r = q / p; if (p < 0) { if (r > t1) { ok = false; break; } if (r > t0) t0 = r; } else { if (r < t0) { ok = false; break; } if (r < t1) t1 = r; } }
    }
    if (!ok || t1 - t0 <= 1e-9) continue;
    // the segment is inside for t0 < t < t1: it enters at t0 > 0, leaves at t1 < 1
    if (t0 > 1e-9) { const p = [g.ax + dx * t0, g.ay + dy * t0]; out.push({ p, side: sideAt(p, dx, dy) }); }
    if (t1 < 1 - 1e-9) { const p = [g.ax + dx * t1, g.ay + dy * t1]; out.push({ p, side: sideAt(p, dx, dy) }); }
  }
  return out;
}

// distance from a segment list to a box (0 when a segment enters it)
function distSegsBox(segs, b) {
  const x0 = b.x, y0 = b.y, x1 = b.x + b.w, y1 = b.y + b.h;
  const sides = [
    { ax: x0, ay: y0, bx: x1, by: y0, len: b.w }, { ax: x1, ay: y0, bx: x1, by: y1, len: b.h },
    { ax: x1, ay: y1, bx: x0, by: y1, len: b.w }, { ax: x0, ay: y1, bx: x0, by: y0, len: b.h },
  ].filter(g => g.len > 0);
  let best = Infinity;
  for (const g of segs) {
    // quick reject by the segment's box
    const gx0 = Math.min(g.ax, g.bx), gx1 = Math.max(g.ax, g.bx), gy0 = Math.min(g.ay, g.by), gy1 = Math.max(g.ay, g.by);
    const lb = Math.hypot(Math.max(x0 - gx1, 0, gx0 - x1), Math.max(y0 - gy1, 0, gy0 - y1));
    if (lb >= best) continue;
    if (insideIntervals([g], b, 0).length) return 0;
    for (const sd of sides) {
      best = Math.min(best, closestOnSeg(sd, [g.ax, g.ay]).d, closestOnSeg(sd, [g.bx, g.by]).d,
        closestOnSeg(g, [sd.ax, sd.ay]).d, closestOnSeg(g, [sd.bx, sd.by]).d);
    }
  }
  return best;
}

// Bridge crossing hops: a short detour that leaves a straight run and comes
// back onto the same line (mermaid's `A6,6` arcs; any bump shape) becomes the
// straight chord, so a hop neither breaks a straight run nor adds length.
function dehop(segs) {
  let hops = 0;
  const out = [];
  let k = 0;
  // the segment index at which a detour starting at segs[from] (leaving the
  // line through `cur` at point (sx, sy)) is back on that line, or -1
  //
  // A bridge (flowmaid from B11 on: neighbouring hops joined) leaves the line
  // on a quarter circle of radius r <= HOP_DEV, runs level at r off the line
  // for any length and comes back down on another quarter circle. Its level
  // top does not count against HOP_SPAN, but then the way up and the way
  // down must be quarter circles (as far along the line as off it): a jog to
  // a neighbouring track and back is no hop.
  const bumpEnd = (cur, ux, uy, from, sx, sy) => {
    let span = 0, dev = 0, lvl = -1, lvlEnd = null;
    for (let j = from; j < segs.length; j++) {
      const h = segs[j];
      const offA = Math.abs((h.ax - cur.ax) * uy - (h.ay - cur.ay) * ux);
      const off = Math.abs((h.bx - cur.ax) * uy - (h.by - cur.ay) * ux);
      const along = ((h.bx - h.ax) * ux + (h.by - h.ay) * uy) / h.len;
      const level = off >= 1.0 && Math.abs(off - offA) < 0.02 && along > 0.9999;
      if (level) {
        if (lvl < 0) {
          const rise = (h.ax - sx) * ux + (h.ay - sy) * uy;
          if (Math.abs(rise - offA) > HOP_QUARTER_TOL) return -1; // not a quarter circle up
          lvl = offA;
        } else if (Math.abs(offA - lvl) > 0.02) return -1; // a second level, at another height
        lvlEnd = [h.bx, h.by];
      } else span += h.len;
      if (span > HOP_SPAN) return -1;
      dev = Math.max(dev, off);
      if (dev > HOP_DEV) return -1;
      if (off < 0.3) {
        if (dev < 1.0) return -1; // never left the line: not a detour
        if (lvlEnd) {
          const fall = (h.bx - lvlEnd[0]) * ux + (h.by - lvlEnd[1]) * uy;
          if (Math.abs(fall - lvl) > HOP_QUARTER_TOL) return -1; // not a quarter circle down
        }
        return (h.bx - sx) * ux + (h.by - sy) * uy > 0.5 ? j : -1;
      }
    }
    return -1;
  };
  while (k < segs.length) {
    const cur = { ...segs[k] };
    out.push(cur);
    k++;
    // extend `cur` across a chain of bumps (hops may sit back to back) that
    // is followed by the line carrying straight on
    while (cur.len >= 2 && k < segs.length) {
      const ux = (cur.bx - cur.ax) / cur.len, uy = (cur.by - cur.ay) / cur.len;
      let from = k, sx = cur.bx, sy = cur.by, n = 0, end = -1;
      for (;;) {
        const j = bumpEnd(cur, ux, uy, from, sx, sy);
        if (j < 0) break;
        n++;
        const nx = segs[j + 1];
        if (nx && nx.len >= 1) {
          const d = ((nx.bx - nx.ax) * ux + (nx.by - nx.ay) * uy) / nx.len;
          const offn = Math.abs((nx.bx - cur.ax) * uy - (nx.by - cur.ay) * ux);
          if (d > 0.9995 && offn < 0.3) { end = j + 1; break; }
        }
        from = j + 1; sx = segs[j].bx; sy = segs[j].by;
      }
      if (end < 0) break;
      cur.bx = segs[end].bx; cur.by = segs[end].by;
      cur.len = Math.hypot(cur.bx - cur.ax, cur.by - cur.ay);
      k = end + 1;
      hops += n;
    }
  }
  let s = 0;
  for (const g of out) { g.s0 = s; s += g.len; }
  return { segs: out, hops };
}

// maximal straight runs of a segment list (lengths)
function straightRuns(segs) {
  const runs = [];
  let run = null; // {ax, ay, ux, uy, len}
  const cosTol = Math.cos(STRAIGHT_ANG * Math.PI / 180);
  for (let k = 0; k < segs.length; k++) {
    const g = segs[k];
    const vx = (g.bx - g.ax) / g.len, vy = (g.by - g.ay) / g.len;
    const connected = k > 0 && Math.hypot(segs[k - 1].bx - g.ax, segs[k - 1].by - g.ay) < 1e-6;
    if (run && connected) {
      const off = Math.abs((g.bx - run.ax) * run.uy - (g.by - run.ay) * run.ux);
      if (vx * run.ux + vy * run.uy >= cosTol && off <= STRAIGHT_OFF) { run.len += g.len; run.bx = g.bx; run.by = g.by; continue; }
    }
    if (run) runs.push(run);
    run = { ax: g.ax, ay: g.ay, bx: g.bx, by: g.by, ux: vx, uy: vy, len: g.len };
  }
  if (run) runs.push(run);
  return runs;
}

// t-interval of segment a within eps of segment b (distance to a convex set
// is convex along a line, so the set is one interval), or null
function nearInterval(a, b, eps) {
  const d = t => closestOnSeg(b, [a.ax + (a.bx - a.ax) * t, a.ay + (a.by - a.ay) * t]).d;
  // minimise the convex function by ternary search
  let lo = 0, hi = 1;
  for (let it = 0; it < 40; it++) {
    const m1 = lo + (hi - lo) / 3, m2 = hi - (hi - lo) / 3;
    if (d(m1) <= d(m2)) hi = m2; else lo = m1;
  }
  const tm = (lo + hi) / 2;
  let best = tm, dm = d(tm);
  for (const t of [0, 1]) { const v = d(t); if (v < dm) { dm = v; best = t; } }
  if (dm > eps) return null;
  let a0 = 0, a1 = best;
  if (d(0) > eps) { for (let it = 0; it < 40; it++) { const m = (a0 + a1) / 2; if (d(m) > eps) a0 = m; else a1 = m; } } else a1 = 0;
  let b0 = best, b1 = 1;
  if (d(1) > eps) { for (let it = 0; it < 40; it++) { const m = (b0 + b1) / 2; if (d(m) > eps) b1 = m; else b0 = m; } } else b0 = 1;
  return [a1, b0];
}

// which side of polyline B the point p lies on (+1 / -1), or 0 when p is
// beyond one of B's ends or on B
function sideOf(segsB, p) {
  const c = closestOnSegs(segsB, p);
  if (!c) return 0;
  const last = segsB.length - 1;
  if ((c.k === 0 && c.t <= 1e-9) || (c.k === last && c.t >= 1 - 1e-9)) {
    // nearest point is a terminal of B: p is "beyond the end" unless it projects onto the end segment
    const g = segsB[c.k];
    const t = ((p[0] - g.ax) * (g.bx - g.ax) + (p[1] - g.ay) * (g.by - g.ay)) / (g.len * g.len);
    if (t < -1e-9 || t > 1 + 1e-9) return 0;
  }
  // at a corner of B the nearest point belongs to two segments: p lies in the
  // corner's outer wedge, on the same side of both, but exactly on the
  // extension of one of them that one says 0 - take the clearer verdict
  let off = 0;
  for (let k = Math.max(0, c.k - 1); k <= Math.min(last, c.k + 1); k++) {
    const g = segsB[k];
    if (closestOnSeg(g, p).d > c.d + 1e-7) continue;
    const o = ((g.bx - g.ax) * (p[1] - g.ay) - (g.by - g.ay) * (p[0] - g.ax)) / g.len;
    if (Math.abs(o) > Math.abs(off)) off = o;
  }
  if (Math.abs(off) < 1e-6) return 0;
  return off > 0 ? 1 : -1;
}

// crossings of edge A with edge B: A goes from one side of B to the other.
// Shared trunks, touches and common ends are contacts without a side change.
function crossingsBetween(A, B) {
  const iv = [];
  for (const a of A.segs) {
    const ab = [Math.min(a.ax, a.bx) - EPS_CONTACT, Math.min(a.ay, a.by) - EPS_CONTACT, Math.max(a.ax, a.bx) + EPS_CONTACT, Math.max(a.ay, a.by) + EPS_CONTACT];
    for (const b of B.segs) {
      if (Math.max(b.ax, b.bx) < ab[0] || Math.min(b.ax, b.bx) > ab[2] || Math.max(b.ay, b.by) < ab[1] || Math.min(b.ay, b.by) > ab[3]) continue;
      const r = nearInterval(a, b, EPS_CONTACT);
      if (r) iv.push([a.s0 + r[0] * a.len, a.s0 + r[1] * a.len]);
    }
  }
  if (!iv.length) return [];
  const contacts = mergeIntervals(iv, 0.75);
  const LA = A.len, out = [];
  for (const [s0, s1] of contacts) {
    if (s0 - DELTA_SIDE < 0 || s1 + DELTA_SIDE > LA) continue; // A starts or ends on B
    const p0 = pointAt(A.segs, s0 - DELTA_SIDE), p1 = pointAt(A.segs, s1 + DELTA_SIDE);
    const sd0 = sideOf(B.segs, p0), sd1 = sideOf(B.segs, p1);
    if (sd0 !== 0 && sd1 !== 0 && sd0 !== sd1) out.push(pointAt(A.segs, (s0 + s1) / 2));
  }
  return out;
}

// ------------------------------------------------------------------ flowmaid SVG
function shapeBox(el, m) { // bbox of one SVG shape element under matrix m, or null
  const A = el.attrs;
  let pts = null;
  if (el.name === 'rect') {
    const x = num(A.x), y = num(A.y), w = num(A.width), h = num(A.height);
    if (!(w > 0 || h > 0)) return null;
    pts = [[x, y], [x + w, y], [x, y + h], [x + w, y + h]];
  } else if (el.name === 'circle') {
    const cx = num(A.cx), cy = num(A.cy), r = num(A.r);
    pts = [[cx - r, cy - r], [cx + r, cy - r], [cx - r, cy + r], [cx + r, cy + r]];
  } else if (el.name === 'ellipse') {
    const cx = num(A.cx), cy = num(A.cy), rx = num(A.rx), ry = num(A.ry);
    pts = [[cx - rx, cy - ry], [cx + rx, cy - ry], [cx - rx, cy + ry], [cx + rx, cy + ry]];
  } else if (el.name === 'polygon' || el.name === 'polyline') {
    const v = (A.points || '').split(/[\s,]+/).filter(Boolean).map(Number);
    pts = [];
    for (let k = 0; k + 1 < v.length; k += 2) pts.push([v[k], v[k + 1]]);
  } else if (el.name === 'line') {
    pts = [[num(A.x1), num(A.y1)], [num(A.x2), num(A.y2)]];
  } else if (el.name === 'path') {
    pts = [].concat(...flattenPath(A.d || '', null));
  }
  if (!pts || !pts.length) return null;
  return bboxOfPoints(pts.map(p => apply(m, p)));
}

function parseFlowmaid(doc, warn) {
  const svg = doc.children.find(c => c.name === 'svg');
  if (!svg) throw new Error('no <svg> root');
  const vb = (svg.attrs.viewBox || '').split(/[\s,]+/).map(Number);
  const model = { engine: 'fm', W: vb[2] || num(svg.attrs.width), H: vb[3] || num(svg.attrs.height), frames: [], nodes: [], edges: [], labels: [], unknown: 0 };
  const chips = [], titleChips = [];
  let pendingChip = null, pendingTitle = null;
  const I = [1, 0, 0, 1, 0, 0];
  const addNode = g => {
    let box = null;
    const walk = (el) => {
      for (const c of el.children) {
        if (c.name === 'text' || c.name === 'title') continue;
        if (c.name === 'g' || c.name === 'a') { walk(c); continue; }
        box = boxUnion(box, shapeBox(c, I));
      }
    };
    walk(g);
    const title = g.children.find(c => c.name === 'title');
    const txt = g.children.filter(c => c.name === 'text').map(textOf).join('\n');
    if (!box) { warn('node group without a shape'); return; }
    model.nodes.push({ id: null, title: title ? textOf(title) : txt, box });
  };
  for (const el of svg.children) {
    const A = el.attrs;
    if (el.name === 'defs' || el.name === 'title' || el.name === 'desc' || el.name === 'style') continue;
    if (el.name === 'rect') {
      if (A.x === undefined && A.y === undefined) continue; // canvas background
      const box = { x: num(A.x), y: num(A.y), w: num(A.width), h: num(A.height) };
      if (A['fill-opacity'] !== undefined) { pendingTitle = { box, lines: [] }; titleChips.push(pendingTitle); pendingChip = null; continue; }
      if (A['stroke-width'] === '1.4' && !model.edges.length && !model.nodes.length) { model.frames.push({ id: null, title: '', box }); continue; }
      if (A.stroke !== undefined && A.rx === '4') { pendingChip = { box, text: '' }; chips.push(pendingChip); pendingTitle = null; continue; }
      model.unknown++;
      continue;
    }
    if (el.name === 'text') {
      if (pendingChip) { pendingChip.text = textOf(el); pendingChip = null; }
      else if (pendingTitle) pendingTitle.lines.push(textOf(el));
      continue;
    }
    if (el.name === 'path') {
      const t = el.children.find(c => c.name === 'title');
      if (!t) { model.unknown++; continue; }
      const title = textOf(t);
      const mm = /^(.*?) (→|↔) (.*)$/.exec(title);
      model.edges.push({ title, tFrom: mm ? mm[1] : null, tRest: mm ? mm[3] : null, d: A.d || '', dataId: A['data-id'] || A.id || null, from: null, to: null, label: null });
      continue;
    }
    if (el.name === 'g') { addNode(el); continue; }
    if (el.name === 'a') { for (const c of el.children) if (c.name === 'g') addNode(c); continue; }
    model.unknown++;
  }
  // frame titles: the chip sits at frame + (6, 5)
  for (const tc of titleChips) {
    const f = model.frames.find(f => Math.abs(f.box.x + 6 - tc.box.x) < 0.2 && Math.abs(f.box.y + 5 - tc.box.y) < 0.2 && !f.title);
    if (f) f.title = tc.lines.join('\n');
  }
  // no ids in the SVG: until a structure file names them, a frame goes by its title
  for (const f of model.frames) if (f.title) f.id = f.title.split('\n')[0];
  model.chips = chips;
  return model;
}

// ------------------------------------------------------------------ mermaid SVG
function parseMermaid(doc, warn) {
  const svg = doc.children.find(c => c.name === 'svg');
  if (!svg) throw new Error('no <svg> root');
  const vb = (svg.attrs.viewBox || '').split(/[\s,]+/).map(Number);
  const model = { engine: 'mermaid', W: vb[2], H: vb[3], frames: [], nodes: [], edges: [], labels: [], unknown: 0, chips: [] };
  const prefix = svg.attrs.id ? svg.attrs.id + '-' : '';
  const strip = id => (id && prefix && id.startsWith(prefix) ? id.slice(prefix.length) : id || '');
  const nodeId = dom => {
    const s = strip(dom);
    const m = /^(?:flowchart|state)-(.*)-\d+$/.exec(s);
    return m ? m[1] : s;
  };
  const shapesBox = (el, m, skipLabel) => {
    let box = null;
    const walk = (e, mm) => {
      for (const c of e.children) {
        if (c.name === 'foreignObject' || c.name === 'text' || c.name === 'title' || c.name === 'style') continue;
        const cm = c.attrs.transform ? mul(mm, parseTransform(c.attrs.transform)) : mm;
        if (c.name === 'g') {
          if (skipLabel && (hasClass(c, 'label') || hasClass(c, 'cluster-label'))) continue;
          walk(c, cm); continue;
        }
        box = boxUnion(box, shapeBox(c, cm));
      }
    };
    walk(el, m);
    return box;
  };
  const visit = (el, m) => {
    for (const c of el.children) {
      if (c.name === 'defs' || c.name === 'marker' || c.name === 'style' || c.name === 'title' || c.name === 'desc') continue;
      const cm = c.attrs.transform ? mul(m, parseTransform(c.attrs.transform)) : m;
      const cl = classes(c);
      if (c.name === 'g' && (cl.includes('cluster') || cl.includes('statediagram-cluster')) && !cl.includes('note-cluster')) {
        // the frame is the first rect (state: rect.outer); nested content is visited too
        let rect = null;
        const find = e => { for (const k of e.children) { if (rect) return; if (k.name === 'rect' && (!hasClass(k, 'inner'))) { rect = k; return; } if (k.name === 'g' && !hasClass(k, 'cluster-label')) find(k); } };
        find(c);
        let box = rect ? shapeBox(rect, cm) : null;
        if (!box) box = shapesBox(c, cm, true);
        const lab = c.children.find(k => k.name === 'g' && hasClass(k, 'cluster-label'));
        let title = '';
        if (lab) { const fo = findDesc(lab, 'foreignObject'); title = fo ? fo.text : textOf(lab); }
        if (box) model.frames.push({ id: c.attrs['data-id'] || strip(c.attrs.id), title, box });
        else warn('cluster without a box: ' + c.attrs.id);
        continue;
      }
      if (c.name === 'g' && cl.includes('note-cluster')) continue;
      if (c.name === 'g' && cl.includes('node')) {
        const box = shapesBox(c, cm, true);
        const fo = findDesc(c, 'foreignObject');
        if (box) model.nodes.push({ id: nodeId(c.attrs.id), title: fo ? fo.text : '', box, note: cl.includes('statediagram-note') });
        else warn('node without a shape: ' + c.attrs.id);
        continue;
      }
      if (c.name === 'g' && cl.includes('edgeLabel')) {
        const lab = c.children.find(k => k.name === 'g' && hasClass(k, 'label')) || c;
        const lm = lab.attrs.transform && lab !== c ? mul(cm, parseTransform(lab.attrs.transform)) : cm;
        const fo = findDesc(lab, 'foreignObject');
        let box = null, text = '';
        if (fo) {
          const w = num(fo.attrs.width), h = num(fo.attrs.height);
          const fm = fo.attrs.transform ? mul(lm, parseTransform(fo.attrs.transform)) : lm;
          if (w > 0 && h > 0) { const p = apply(fm, [num(fo.attrs.x), num(fo.attrs.y)]); box = { x: p[0], y: p[1], w, h }; text = fo.text; }
        } else {
          box = shapesBox(lab, lm, false);
          text = textOf(lab);
          if (box && !(box.w > 0 && box.h > 0)) box = null;
        }
        if (box) model.labels.push({ box, text, dataId: lab.attrs['data-id'] || null });
        continue;
      }
      if (c.name === 'path' && (c.attrs['data-edge'] === 'true' || (el.name === 'g' && hasClass(el, 'edgePaths')))) {
        if (cl.includes('edge-thickness-invisible')) continue; // `~~~` link: laid out, never drawn
        const polys = flattenPath(c.attrs.d || '', null).map(pl => pl.map(p => apply(cm, p)));
        model.edges.push({ title: strip(c.attrs.id), dataId: c.attrs['data-id'] || strip(c.attrs.id), polys, from: null, to: null, label: null, note: hasClass(c, 'note-edge') });
        continue;
      }
      if (c.name === 'g' || c.name === 'svg' || c.name === 'a') visit(c, cm);
    }
  };
  visit(svg, [1, 0, 0, 1, 0, 0]);
  return model;
}
function findDesc(el, name) {
  for (const c of el.children) { if (c.name === name) return c; const r = findDesc(c, name); if (r) return r; }
  return null;
}

// ------------------------------------------------------------------ structure
// Give every frame a parent and every node a frame, from the structure file
// when there is one (authoritative), else from the geometry.
function resolveStructure(model, st, warn) {
  const F = model.frames, N = model.nodes;
  for (const f of F) { f.parent = null; f.sub = null; }
  for (const nd of N) nd.frame = null;
  const geomParent = () => {
    for (let i = 0; i < F.length; i++) {
      let best = -1;
      for (let j = 0; j < F.length; j++) {
        if (i === j) continue;
        if (boxContainsBox(F[j].box, F[i].box) && F[j].box.w * F[j].box.h > F[i].box.w * F[i].box.h &&
          (best < 0 || F[j].box.w * F[j].box.h < F[best].box.w * F[best].box.h)) best = j;
      }
      F[i].parent = best < 0 ? null : best;
    }
  };
  const geomFrameOf = nd => {
    let best = -1;
    const c = center(nd.box);
    for (let j = 0; j < F.length; j++) {
      if (boxContainsPoint(F[j].box, c) && (best < 0 || F[j].box.w * F[j].box.h < F[best].box.w * F[best].box.h)) best = j;
    }
    return best < 0 ? null : best;
  };
  model.membership = 'geom';
  if (!st) { geomParent(); for (const nd of N) nd.frame = geomFrameOf(nd); return; }

  // --- nodes
  let matched = 0;
  if (model.engine === 'fm') {
    if (N.length === st.nodes.length) { N.forEach((nd, i) => { nd.id = st.nodes[i].id; nd.sn = i; }); matched = N.length; }
    else warn(`struct has ${st.nodes.length} nodes, the SVG ${N.length}: falling back to geometry`);
  } else {
    const byId = new Map(st.nodes.map((s, i) => [s.id, i]));
    for (const nd of N) { if (byId.has(nd.id)) { nd.sn = byId.get(nd.id); matched++; } }
  }
  // --- frames <-> subgraphs
  const SG = st.subgraphs;
  const desc = SG.map(() => new Set());
  st.nodes.forEach((s, i) => { let k = s.sub; while (k !== null && k !== undefined) { desc[k].add(i); k = SG[k].parent; } });
  const nodeOfSn = new Map(); N.forEach((nd, i) => { if (nd.sn !== undefined) nodeOfSn.set(nd.sn, i); });
  const subOfFrame = new Array(F.length).fill(null);
  const frameOfSub = new Array(SG.length).fill(null);
  if (model.engine !== 'fm') {
    const byId = new Map(SG.map((s, i) => [s.id, i]));
    F.forEach((f, i) => { if (byId.has(f.id)) { subOfFrame[i] = byId.get(f.id); frameOfSub[subOfFrame[i]] = i; } });
  } else if (matched) {
    // smallest frame first; among the subgraphs whose every node lies in the
    // frame take the one with the most nodes, then the deepest, then by title
    const depth = k => { let d = 0; while (SG[k].parent !== null && SG[k].parent !== undefined) { k = SG[k].parent; d++; } return d; };
    const order = F.map((_, i) => i).sort((a, b) => F[a].box.w * F[a].box.h - F[b].box.w * F[b].box.h);
    const plain = s => String(s || '').replace(/<[^>]*>/g, '').replace(/\s+/g, ' ').trim();
    for (const fi of order) {
      let best = -1, bestKey = null;
      for (let k = 0; k < SG.length; k++) {
        if (frameOfSub[k] !== null) continue;
        let ok = true;
        for (const sn of desc[k]) { const ni = nodeOfSn.get(sn); if (ni === undefined || !boxContainsPoint(F[fi].box, center(N[ni].box))) { ok = false; break; } }
        if (!ok) continue;
        const titleHit = plain(F[fi].title) && plain(SG[k].title).startsWith(plain(F[fi].title).replace(/…$/, '')) ? 1 : 0;
        const key = [desc[k].size > 0 ? 1 : 0, titleHit, desc[k].size, depth(k)];
        if (best < 0 || cmpKey(key, bestKey) > 0) { best = k; bestKey = key; }
      }
      if (best >= 0) { subOfFrame[fi] = best; frameOfSub[best] = fi; }
    }
    // scene order, when the dump was made by the same version, must agree
    if (st.scene && st.scene.clusters && st.scene.clusters.length === F.length) {
      const idOf = k => (k === null ? null : SG[k].id);
      const disagree = F.filter((f, i) => idOf(subOfFrame[i]) !== st.scene.clusters[i].id).length;
      if (disagree) model.notes.push(`frame order differs from struct.scene for ${disagree} frame(s) (dump from another version?)`);
    }
  }
  F.forEach((f, i) => { if (subOfFrame[i] !== null) { f.sub = subOfFrame[i]; f.id = SG[subOfFrame[i]].id; } else warn(`frame ${i} (${f.id || f.title || '?'}) matches no subgraph`); });
  SG.forEach((s, k) => { if (frameOfSub[k] === null && desc[k].size) warn(`subgraph ${s.id} has no frame in the SVG`); });
  // parents
  let anyGeomParent = false;
  F.forEach((f, i) => {
    if (f.sub === null) { anyGeomParent = true; return; }
    let p = SG[f.sub].parent;
    while (p !== null && p !== undefined && frameOfSub[p] === null) p = SG[p].parent;
    f.parent = p === null || p === undefined ? null : frameOfSub[p];
  });
  if (anyGeomParent) {
    const keep = F.map(f => f.parent);
    geomParent();
    F.forEach((f, i) => { if (f.sub !== null) f.parent = keep[i]; });
  }
  // membership
  let geomNodes = 0;
  for (const nd of N) {
    if (nd.sn !== undefined) {
      let k = st.nodes[nd.sn].sub;
      while (k !== null && k !== undefined && frameOfSub[k] === null) k = SG[k].parent;
      nd.frame = k === null || k === undefined ? null : frameOfSub[k];
    } else { nd.frame = geomFrameOf(nd); geomNodes++; }
  }
  model.membership = geomNodes === 0 ? 'struct' : (geomNodes === N.length ? 'geom' : `struct+${geomNodes}geom`);
  model.frameOfSub = frameOfSub;
  model.nodeOfSn = nodeOfSn;
}
function cmpKey(a, b) { for (let i = 0; i < a.length; i++) { if (a[i] !== b[i]) return a[i] - b[i]; } return 0; }

// frames holding an end: {n: node index} -> its frame chain; {f: frame index} -> itself + ancestors
function holders(model, end) {
  const out = new Set();
  if (!end) return out;
  let k = end.n !== undefined ? model.nodes[end.n].frame : end.f;
  while (k !== null && k !== undefined) { out.add(k); k = model.frames[k].parent; }
  return out;
}
function geomEnd(model, p) {
  let bn = -1, bd = Infinity;
  model.nodes.forEach((nd, i) => { const d = distPointBox(p, nd.box); if (d < bd) { bd = d; bn = i; } });
  if (bn >= 0 && bd <= END_NODE_TOL) return { n: bn, how: 'geom' };
  let bf = -1, fd = Infinity;
  model.frames.forEach((f, i) => { const d = distPointBoxBorder(p, f.box); if (d < fd || (d === fd && bf >= 0 && f.box.w * f.box.h < model.frames[bf].box.w * model.frames[bf].box.h)) { fd = d; bf = i; } });
  if (bf >= 0 && fd <= END_FRAME_TOL) return { f: bf, how: 'geom' };
  if (bn >= 0 && bd <= 40) return { n: bn, how: 'geom-far' };
  return null;
}
function endName(model, e) {
  if (!e) return '?';
  if (e.n !== undefined) return model.nodes[e.n].id || `#${e.n}`;
  return '[' + (model.frames[e.f].id || `frame${e.f}`) + ']';
}

function resolveEdges(model, st, warn) {
  const E = model.edges;
  for (const e of E) {
    if (!e.polys) e.polys = flattenPath(e.d, null);
    const raw = toSegs(e.polys);
    const dh = dehop(raw);
    e.segs = dh.segs; e.hops = dh.hops; e.len = segsLength(e.segs); e.bbox = e.segs.length ? segsBBox(e.segs) : { x: 0, y: 0, w: 0, h: 0 };
    e.p0 = e.segs.length ? [e.segs[0].ax, e.segs[0].ay] : null;
    e.p1 = e.segs.length ? [e.segs[e.segs.length - 1].bx, e.segs[e.segs.length - 1].by] : null;
  }
  const structEnd = (x) => {
    if (x.n !== undefined) { const ni = model.nodeOfSn ? model.nodeOfSn.get(x.n) : undefined; return ni === undefined ? null : { n: ni, how: 'struct' }; }
    const fi = model.frameOfSub ? model.frameOfSub[x.s] : null;
    return fi === null || fi === undefined ? null : { f: fi, how: 'struct' };
  };
  let usedStruct = false;
  if (st && model.engine === 'fm' && model.nodeOfSn && model.nodeOfSn.size === st.nodes.length) {
    const drawn = st.edges.filter(e => e.drawn !== false);
    if (drawn.length === E.length) {
      E.forEach((e, i) => { e.from = structEnd(drawn[i].from); e.to = structEnd(drawn[i].to); e.hasLabel = drawn[i].label !== null && drawn[i].label !== ''; });
      usedStruct = true;
    } else warn(`struct has ${drawn.length} drawn edges, the SVG ${E.length}: edge ends from geometry`);
  }
  if (model.engine !== 'fm') {
    // flowchart ids: L_<from>_<to>_<n>
    const nodeIds = new Map(); model.nodes.forEach((nd, i) => { if (nd.id) nodeIds.set(nd.id, { n: i, how: 'id' }); });
    const frameIds = new Map(); model.frames.forEach((f, i) => { if (f.id) frameIds.set(f.id, { f: i, how: 'id' }); });
    const look = id => nodeIds.get(id) || frameIds.get(id) || null;
    for (const e of E) {
      const m = /^L_(.*)_(\d+)$/.exec(e.dataId || '');
      if (!m) continue;
      const body = m[1];
      const cands = [];
      for (let k = body.indexOf('_'); k >= 0; k = body.indexOf('_', k + 1)) {
        const a = look(body.slice(0, k)), b = look(body.slice(k + 1));
        if (a && b) cands.push([a, b]);
      }
      if (cands.length === 1) { e.from = cands[0][0]; e.to = cands[0][1]; }
      else if (cands.length > 1 && e.p0) { // several splits fit: take the one nearest the drawn ends
        const cost = ([a, b]) => endDist(model, a, e.p0) + endDist(model, b, e.p1);
        cands.sort((x, y) => cost(x) - cost(y)); e.from = cands[0][0]; e.to = cands[0][1];
      }
    }
  }
  for (const e of E) {
    if (!e.p0) { warn(`edge "${e.title}" is drawn as a zero-length path`); continue; }
    if (!e.from) { e.from = geomEnd(model, e.p0); if (!e.from) warn(`edge "${e.title}": start end unresolved`); }
    if (!e.to) { e.to = geomEnd(model, e.p1); if (!e.to) warn(`edge "${e.title}": far end unresolved`); }
  }
  model.edgeEnds = usedStruct ? 'struct' : (model.engine === 'fm' ? 'geom' : (E.every(e => e.from && e.from.how === 'id' && e.to && e.to.how === 'id') ? 'id' : 'id+geom'));

  // labels
  if (model.engine === 'fm') {
    const chips = model.chips;
    let labelled;
    if (usedStruct) labelled = E.filter(e => e.hasLabel);
    else labelled = E.filter(e => e.tRest !== null && /: /.test(e.tRest));
    if (labelled.length === chips.length) labelled.forEach((e, i) => { e.label = { box: chips[i].box, text: chips[i].text }; });
    else {
      warn(`${chips.length} label chips for ${labelled.length} labelled edges: chips attached to the nearest edge`);
      for (const c of chips) {
        let best = null, bd = Infinity;
        for (const e of E) { if (e.label || !e.segs.length) continue; const d = closestOnSegs(e.segs, center(c.box)).d; if (d < bd) { bd = d; best = e; } }
        if (best) best.label = { box: c.box, text: c.text };
      }
    }
  } else {
    const byId = new Map(E.map(e => [e.dataId, e]));
    for (const l of model.labels) {
      let e = l.dataId ? byId.get(l.dataId) : null;
      if (!e) {
        let bd = Infinity;
        for (const x of E) { if (x.label || !x.segs.length) continue; const d = closestOnSegs(x.segs, center(l.box)).d; if (d < bd) { bd = d; e = x; } }
        if (e) warn(`label "${l.text}" attached to the nearest edge "${e.title}"`);
      }
      if (e) e.label = { box: l.box, text: l.text };
    }
  }
}
function endDist(model, end, p) {
  return end.n !== undefined ? distPointBox(p, model.nodes[end.n].box) : distPointBoxBorder(p, model.frames[end.f].box);
}

// ------------------------------------------------------------------ metrics
function inferDir(model) {
  let v = 0, h = 0;
  for (const e of model.edges) {
    if (!e.from || !e.to || e.from.n === undefined || e.to.n === undefined) continue;
    const a = center(model.nodes[e.from.n].box), b = center(model.nodes[e.to.n].box);
    if (Math.abs(b[1] - a[1]) >= Math.abs(b[0] - a[0])) v += (b[1] >= a[1] ? 1 : -1); else h += (b[0] >= a[0] ? 1 : -1);
  }
  if (Math.abs(v) >= Math.abs(h)) return v >= 0 ? 'TD' : 'BT';
  return h >= 0 ? 'LR' : 'RL';
}

function measure(model, dir) {
  const F = model.frames, N = model.nodes, E = model.edges;
  const vertical = dir === 'TD' || dir === 'BT' || dir === 'TB';
  const lo = b => (vertical ? b.y : b.x), ext = b => (vertical ? b.h : b.w);
  const M = { detail: { frames: [], edges: [], labels: [], crossings: [], xruns: [], sib: [] } };
  M.W = model.W; M.H = model.H; M.area = model.W * model.H; M.aspect = model.W / model.H;
  M.nodes = N.length; M.edges = E.length; M.frames = F.length;
  M.node_area = N.reduce((a, nd) => a + nd.box.w * nd.box.h, 0);
  M.fill = M.area > 0 ? M.node_area / M.area : NaN;
  const unit = N.length ? Math.sqrt(M.node_area / N.length) : NaN; // side of the average node box
  M.top_frames = F.filter(f => f.parent === null).length;

  // --- compactness
  const members = F.map(() => []);
  N.forEach((nd, i) => { let k = nd.frame; while (k !== null && k !== undefined) { members[k].push(i); k = F[k].parent; } });
  const canvasExt = vertical ? model.H : model.W;
  let cSum = 0, cN = 0, cMax = 0, cMaxId = '', spanMax = 0, spanMaxId = '';
  F.forEach((f, i) => {
    const fa = f.box.w * f.box.h;
    const na = members[i].reduce((s, n) => s + N[n].box.w * N[n].box.h, 0);
    const compact = na > 0 ? fa / na : NaN;
    const span = ext(f.box) / canvasExt;
    // foreign nodes lying in the frame (a layout defect; none expected)
    const foreign = N.filter((nd, n) => !members[i].includes(n) && boxContainsPoint(f.box, center(nd.box))).length;
    M.detail.frames.push({ id: f.id || `frame${i}`, parent: f.parent === null ? '-' : (F[f.parent].id || `frame${f.parent}`), x: f.box.x, y: f.box.y, w: f.box.w, h: f.box.h, members: members[i].length, node_area: na, compact, span, foreign_nodes: foreign });
    if (na > 0) { cSum += compact; cN++; if (compact > cMax) { cMax = compact; cMaxId = f.id || `frame${i}`; } }
    if (span > spanMax) { spanMax = span; spanMaxId = f.id || `frame${i}`; }
  });
  M.compact_mean = cN ? cSum / cN : NaN; M.compact_max = cN ? cMax : NaN; M.compact_max_id = cMaxId || '-';
  M.span_max = F.length ? spanMax : NaN; M.span_max_id = spanMaxId || '-';
  M.frame_foreign_nodes = M.detail.frames.reduce((s, f) => s + f.foreign_nodes, 0);

  // --- sibling frames along the flow
  M.sib_pairs = 0; M.sib_side = 0; M.sib_stacked = 0; M.sib_stagger = 0; M.sib_side_linked = 0; M.frame_overlap = 0;
  const holds = E.map(e => [holders(model, e.from), holders(model, e.to)]);
  const linked = (i, j) => holds.some(([a, b]) => (a.has(i) && b.has(j)) || (a.has(j) && b.has(i)));
  for (let i = 0; i < F.length; i++) for (let j = i + 1; j < F.length; j++) {
    if (F[i].parent !== F[j].parent) continue;
    M.sib_pairs++;
    const a0 = lo(F[i].box), a1 = a0 + ext(F[i].box), b0 = lo(F[j].box), b1 = b0 + ext(F[j].box);
    const ov = Math.min(a1, b1) - Math.max(a0, b0);
    let kind;
    const link = linked(i, j);
    if (ov > SIDE_MIN) {
      M.sib_side++; kind = 'side';
      if (link) M.sib_side_linked++;
      const contains = (a0 <= b0 + 2 && a1 >= b1 - 2) || (b0 <= a0 + 2 && b1 >= a1 - 2);
      if (!contains) { M.sib_stagger++; kind = 'side(stagger)'; }
    } else { M.sib_stacked++; kind = 'stacked'; }
    const o = boxOverlap(F[i].box, F[j].box);
    if (o[0] > OVERLAP_MIN && o[1] > OVERLAP_MIN) { M.frame_overlap++; kind += '+overlap'; }
    M.detail.sib.push({ a: F[i].id || `frame${i}`, b: F[j].id || `frame${j}`, kind, flow_overlap: ov, linked: link ? 'edge' : '-' });
  }

  // --- edges: length, straight runs, runs through foreign frames, node hits
  M.edge_len = 0; M.straight_max = 0; M.straight_max_edge = '-'; M.bends = 0; M.hops = 0;
  M.bends_max = 0; M.bends_max_edge = '-'; M.detour_max = NaN; M.detour_max_edge = '-';
  M.lateral_x = 0; M.lateral_edges = 0; M.flow_x = 0;
  M.detail.border = [];
  M.xframe_runs = 0; M.xframe_len = 0; M.xframe_edges = 0; M.node_hits = 0;
  const lanesH = [], lanesV = [];
  E.forEach((e, ei) => {
    const name = `${endName(model, e.from)}>${endName(model, e.to)}`;
    e.name = name;
    M.edge_len += e.len; M.hops += e.hops;
    const runs = straightRuns(e.segs);
    let smax = 0;
    for (const r of runs) if (r.len > smax) smax = r.len;
    if (smax > M.straight_max) { M.straight_max = smax; M.straight_max_edge = name; }
    const bends = Math.max(0, runs.filter(r => r.len >= BEND_RUN_MIN).length - 1);
    M.bends += bends;
    if (bends > M.bends_max) { M.bends_max = bends; M.bends_max_edge = name; }
    // detour: drawn length over the Manhattan distance of the two drawn ends
    let detour = NaN;
    if (e.segs.length && e.len >= DETOUR_MIN_LEN) {
      const man = Math.abs(e.p1[0] - e.p0[0]) + Math.abs(e.p1[1] - e.p0[1]);
      detour = man > 1 ? e.len / man : NaN;
      if (!Number.isNaN(detour) && !(detour <= M.detour_max)) { M.detour_max = detour; M.detour_max_edge = name; }
    }
    // crossings of the border of a frame that holds exactly one end: through
    // a lateral side (across the flow) or a flow-facing one
    let lat = 0, flw = 0;
    if (e.from && e.to && e.segs.length) {
      const hf = holders(model, e.from), ht = holders(model, e.to);
      F.forEach((f, fi) => {
        if (hf.has(fi) === ht.has(fi)) return;
        for (const c of borderCrossings(e.segs, f.box)) {
          const lateral = vertical ? (c.side === 'L' || c.side === 'R') : (c.side === 'T' || c.side === 'B');
          if (lateral) lat++; else flw++;
          M.detail.border.push({ edge: name, frame: f.id || `frame${fi}`, side: c.side, kind: lateral ? 'lateral' : 'flow', at: c.p });
        }
      });
    }
    M.lateral_x += lat; M.flow_x += flw; if (lat) M.lateral_edges++;
    for (const r of runs) {
      if (r.len < LANE_RUN_MIN) continue;
      if (Math.abs(r.by - r.ay) <= Math.max(0.5, r.len * 0.0175)) lanesH.push({ c: (r.ay + r.by) / 2, a: Math.min(r.ax, r.bx), b: Math.max(r.ax, r.bx), ei });
      else if (Math.abs(r.bx - r.ax) <= Math.max(0.5, r.len * 0.0175)) lanesV.push({ c: (r.ax + r.bx) / 2, a: Math.min(r.ay, r.by), b: Math.max(r.ay, r.by), ei });
    }
    // foreign frames: hold neither end; only the outermost of a foreign nest
    const own = new Set([...holders(model, e.from), ...holders(model, e.to)]);
    const unresolved = !e.from || !e.to;
    let xlenIv = [], xruns = 0;
    if (!unresolved) {
      F.forEach((f, fi) => {
        if (own.has(fi)) return;
        if (f.parent !== null && !own.has(f.parent)) return; // parent is foreign too: counted there
        const iv = insideIntervals(e.segs, f.box, FRAME_INSET).filter(v => v[1] - v[0] >= RUN_MIN);
        for (const v of iv) { xruns++; xlenIv.push(v); M.detail.xruns.push({ edge: name, frame: f.id || `frame${fi}`, len: v[1] - v[0], from: pointAt(e.segs, v[0]), to: pointAt(e.segs, v[1]), ei, s0: v[0], s1: v[1] }); }
      });
    }
    const xlen = mergeIntervals(xlenIv, 0).reduce((s, v) => s + (v[1] - v[0]), 0);
    M.xframe_runs += xruns; M.xframe_len += xlen; if (xruns) M.xframe_edges++;
    let hits = 0;
    N.forEach((nd, ni) => {
      if ((e.from && e.from.n === ni) || (e.to && e.to.n === ni)) return;
      const b = e.bbox; const o = boxOverlap(b, nd.box); if (o[0] < 0 || o[1] < 0) return;
      const iv = insideIntervals(e.segs, nd.box, 1.0).filter(v => v[1] - v[0] >= RUN_MIN);
      if (iv.length) hits++;
    });
    M.node_hits += hits;
    M.detail.edges.push({ edge: name, len: e.len, straight_max: smax, bends, detour: Number.isNaN(detour) ? '-' : detour, hops: e.hops, xframe_runs: xruns, xframe_len: xlen, node_hits: hits, lateral_x: lat, flow_x: flw, ends: `${e.from ? e.from.how : '?'}/${e.to ? e.to.how : '?'}` });
  });

  M.edge_len_n = unit > 0 ? M.edge_len / unit : NaN;

  // --- parallel lanes: axis-aligned straight runs of different edges side by side
  M.lanes_max = E.length ? 1 : 0; M.lane_gap_min = Infinity; M.lanes_at = '-';
  for (const [runs, axis] of [[lanesH, 'y'], [lanesV, 'x']]) {
    for (let i = 0; i < runs.length; i++) for (let j = i + 1; j < runs.length; j++) {
      const p = runs[i], q = runs[j];
      if (p.ei === q.ei) continue;
      const gap = Math.abs(p.c - q.c);
      if (gap <= 0.5 || gap >= M.lane_gap_min) continue;
      if (Math.min(p.b, q.b) - Math.max(p.a, q.a) >= LANE_RUN_MIN) M.lane_gap_min = gap;
    }
    // the most lines one perpendicular cut crosses with no gap wider than LANE_PITCH
    const cuts = [];
    for (const r of runs) cuts.push(r.a + 0.25, r.b - 0.25);
    for (const x of cuts) {
      const cs = runs.filter(r => r.a <= x && x <= r.b).map(r => r.c).sort((u, v) => u - v);
      let n = 0, prev = -Infinity, start = 0;
      for (const c of cs) {
        if (c - prev <= 0.5) continue; // the same line twice
        if (c - prev <= LANE_PITCH) n++; else { n = 1; start = c; }
        prev = c;
        if (n > M.lanes_max) { M.lanes_max = n; M.lanes_at = `${axis === 'y' ? 'x' : 'y'}=${x.toFixed(0)},${axis}=${start.toFixed(0)}..${c.toFixed(0)}`; }
      }
    }
  }
  if (M.lane_gap_min === Infinity) M.lane_gap_min = NaN;

  // --- crossings
  M.crossings = 0;
  for (let i = 0; i < E.length; i++) for (let j = i + 1; j < E.length; j++) {
    const a = E[i], b = E[j];
    if (!a.segs.length || !b.segs.length) continue;
    const o = boxOverlap(a.bbox, b.bbox);
    if (o[0] < -1 || o[1] < -1) continue;
    const pts = crossingsBetween(a, b);
    // the test is not symmetric at ends (A ending on B is a touch): ask both ways, keep the smaller
    const pts2 = pts.length ? crossingsBetween(b, a) : [];
    const use = pts2.length < pts.length ? pts2 : pts;
    for (const p of use) { M.crossings++; M.detail.crossings.push({ a: a.name, b: b.name, x: p[0], y: p[1] }); }
  }

  // --- labels
  const L = E.filter(e => e.label && e.segs.length); // a zero-length path (warned about) carries no label position
  M.lbl_n = L.length;
  let offSum = 0, offMax = 0, endSum = 0, endMax = 0, detMax = 0, gapMin = Infinity;
  M.lbl_lbl = 0; M.lbl_node = 0; M.lbl_struck = 0; M.lbl_crowd = 0; M.lbl_in_end = 0;
  let ownGapMax = 0;
  L.forEach((e, li) => {
    const c = center(e.label.box);
    const cl = closestOnSegs(e.segs, c);
    const off = e.len > 0 ? Math.abs(cl.s - e.len / 2) / e.len : 0;
    const endpx = Math.min(cl.s, e.len - cl.s);
    offSum += off; if (off > offMax) offMax = off;
    endSum += endpx; if (endpx > endMax) endMax = endpx;
    if (cl.d > detMax) detMax = cl.d;
    let onNode = 0, struck = 0, onLbl = 0;
    N.forEach((nd, ni) => {
      if ((e.from && e.from.n === ni) || (e.to && e.to.n === ni)) return;
      const o = boxOverlap(e.label.box, nd.box);
      if (o[0] > OVERLAP_MIN && o[1] > OVERLAP_MIN) onNode++;
    });
    let gap = Infinity;
    for (const x of E) {
      if (x === e || !x.segs.length) continue;
      const o = boxOverlap(e.label.box, x.bbox);
      if (-Math.min(o[0], o[1]) >= gap) continue;
      gap = Math.min(gap, distSegsBox(x.segs, e.label.box));
      if (o[0] < 0 || o[1] < 0) continue;
      if (insideIntervals(x.segs, e.label.box, 1.0).some(v => v[1] - v[0] >= RUN_MIN)) struck++;
    }
    for (let k = li + 1; k < L.length; k++) {
      const o = boxOverlap(e.label.box, L[k].label.box);
      if (o[0] > OVERLAP_MIN && o[1] > OVERLAP_MIN) { M.lbl_lbl++; onLbl++; }
    }
    M.lbl_node += onNode; M.lbl_struck += struck;
    if (gap < gapMin) gapMin = gap;
    if (gap < LBL_NEAR) M.lbl_crowd++;
    // the chip against its own edge, and against the frames that hold exactly one end of it
    const ownGap = distSegsBox(e.segs, e.label.box);
    if (ownGap > ownGapMax) ownGapMax = ownGap;
    let inEnd = '-';
    if (e.from && e.to) {
      const hf = holders(model, e.from), ht = holders(model, e.to);
      F.forEach((f, fi) => {
        if (inEnd !== '-' || hf.has(fi) === ht.has(fi)) return;
        const o = boxOverlap(e.label.box, f.box);
        const onBorder = o[0] > OVERLAP_MIN && o[1] > OVERLAP_MIN && !boxContainsBox(f.box, e.label.box);
        if (boxContainsPoint(f.box, c) || onBorder) inEnd = f.id || `frame${fi}`;
      });
    }
    if (inEnd !== '-') M.lbl_in_end++;
    M.detail.labels.push({ edge: e.name, text: e.label.text, x: c[0], y: c[1], w: e.label.box.w, h: e.label.box.h, edge_len: e.len, s: cl.s, off, end_px: endpx, detach: cl.d, own_gap: ownGap, in_end: inEnd, gap, on_node: onNode, struck, on_label_after: onLbl });
  });
  M.lbl_off_mean = L.length ? offSum / L.length : NaN; M.lbl_off_max = L.length ? offMax : NaN;
  M.lbl_endpx_mean = L.length ? endSum / L.length : NaN; M.lbl_endpx_max = L.length ? endMax : NaN;
  M.lbl_detach_max = L.length ? detMax : NaN;
  M.lbl_gap_min = L.length && gapMin < Infinity ? gapMin : NaN;
  M.lbl_own_gap_max = L.length ? ownGapMax : NaN;

  // --- bands: top-level frames and free nodes grouped by overlapping flow
  // projections, bands in flow order, members by position across the flow
  {
    const items = [];
    F.forEach((f, i) => { if (f.parent === null) items.push({ id: f.id || `frame${i}`, box: f.box }); });
    N.forEach((nd, i) => { if (nd.frame === null || nd.frame === undefined) items.push({ id: nd.id || (nd.title ? nd.title.split('\n')[0] : '') || `#${i}`, box: nd.box }); });
    items.sort((a, b) => lo(a.box) - lo(b.box) || a.id.localeCompare(b.id));
    const bands = [];
    let hi = -Infinity;
    for (const it of items) {
      if (!bands.length || lo(it.box) > hi - SIDE_MIN) { bands.push([it]); hi = lo(it.box) + ext(it.box); }
      else { bands[bands.length - 1].push(it); hi = Math.max(hi, lo(it.box) + ext(it.box)); }
    }
    if (dir === 'BT' || dir === 'RL') bands.reverse();
    const across = b => (vertical ? b.x + b.w / 2 : b.y + b.h / 2);
    M.bands_n = bands.length;
    M.bands = bands.map(b => b.slice().sort((p, q) => across(p.box) - across(q.box) || lo(p.box) - lo(q.box) || p.id.localeCompare(q.id)).map(x => x.id).join(' ')).join(' / ') || '-';
    M.band_sets = bands.map(b => b.map(x => x.id).sort().join(' ')).join(' / ') || '-';
  }
  M.breadth = vertical ? model.W : model.H;
  return M;
}

// ------------------------------------------------------------------ output
const COLUMNS = [
  ['name', 's'], ['engine', 's'], ['dir', 's'], ['W', 0], ['H', 0], ['area', 0], ['aspect', 2], ['fill', 3],
  ['nodes', 0], ['edges', 0], ['frames', 0], ['top_frames', 0],
  ['compact_mean', 2], ['compact_max', 2], ['compact_max_id', 's'], ['span_max', 3], ['span_max_id', 's'],
  ['sib_pairs', 0], ['sib_side', 0], ['sib_stacked', 0], ['sib_stagger', 0], ['sib_side_linked', 0], ['frame_overlap', 0], ['frame_foreign_nodes', 0],
  ['xframe_runs', 0], ['xframe_len', 0], ['xframe_edges', 0], ['node_hits', 0],
  ['crossings', 0], ['hops', 0], ['edge_len', 0], ['edge_len_n', 1], ['straight_max', 0], ['straight_max_edge', 's'], ['bends', 0], ['lanes_max', 0], ['lane_gap_min', 1],
  ['lbl_n', 0], ['lbl_off_mean', 3], ['lbl_off_max', 3], ['lbl_endpx_mean', 0], ['lbl_endpx_max', 0], ['lbl_detach_max', 1],
  ['lbl_lbl', 0], ['lbl_node', 0], ['lbl_struck', 0], ['lbl_gap_min', 1], ['lbl_crowd', 0],
  ['membership', 's'], ['edge_ends', 's'], ['warn', 0],
  // added for B12 (spec 4.2 B12.0), after every earlier column
  ['lateral_x', 0], ['lateral_edges', 0], ['flow_x', 0], ['bends_max', 0], ['bends_max_edge', 's'], ['detour_max', 2], ['detour_max_edge', 's'],
  ['lbl_in_end', 0], ['lbl_own_gap_max', 1], ['breadth', 0], ['bands_n', 0], ['bands', 's'], ['band_sets', 's'],
];
function fmt(v, d) {
  if (d === 's') return String(v ?? '-');
  if (v === null || v === undefined || Number.isNaN(v)) return 'NA';
  return Number(v).toFixed(d);
}
function row(M) { return COLUMNS.map(([k, d]) => fmt(M[k], d)).join('\t'); }
function table(rows, cols) {
  if (!rows.length) return '  (none)\n';
  const f = v => (typeof v === 'number' ? (Number.isInteger(v) ? String(v) : v.toFixed(Math.abs(v) < 10 ? 3 : 1)) : Array.isArray(v) ? `(${v.map(x => x.toFixed(1)).join(',')})` : String(v));
  const data = rows.map(r => cols.map(c => f(r[c])));
  const w = cols.map((c, i) => Math.max(c.length, ...data.map(r => r[i].length)));
  let s = '  ' + cols.map((c, i) => c.padEnd(w[i])).join('  ') + '\n';
  for (const r of data) s += '  ' + r.map((c, i) => c.padEnd(w[i])).join('  ') + '\n';
  return s;
}

function overlaySvg(src, model, M) {
  const o = [];
  o.push('<g id="blockmetrics-overlay" fill="none" style="pointer-events:none">');
  for (const f of model.frames) o.push(`<rect x="${f.box.x.toFixed(1)}" y="${f.box.y.toFixed(1)}" width="${f.box.w.toFixed(1)}" height="${f.box.h.toFixed(1)}" stroke="#0a7" stroke-width="2" stroke-dasharray="6 4"/>`);
  for (const nd of model.nodes) o.push(`<rect x="${nd.box.x.toFixed(1)}" y="${nd.box.y.toFixed(1)}" width="${nd.box.w.toFixed(1)}" height="${nd.box.h.toFixed(1)}" stroke="#0a7" stroke-width="1" stroke-dasharray="2 2"/>`);
  for (const r of M.detail.xruns) {
    const e = model.edges[r.ei];
    const pts = [r.from];
    for (const g of e.segs) { const s = g.s0 + g.len; if (s > r.s0 && s < r.s1) pts.push([g.bx, g.by]); }
    pts.push(r.to);
    o.push(`<polyline points="${pts.map(p => p[0].toFixed(1) + ',' + p[1].toFixed(1)).join(' ')}" stroke="#e00" stroke-width="6" stroke-opacity="0.55"/>`);
  }
  for (const c of M.detail.crossings) o.push(`<circle cx="${c.x.toFixed(1)}" cy="${c.y.toFixed(1)}" r="7" stroke="#f0f" stroke-width="2.5"/>`);
  for (const e of model.edges) {
    if (!e.label) continue;
    const c = center(e.label.box), mid = pointAt(e.segs, e.len / 2);
    o.push(`<rect x="${e.label.box.x.toFixed(1)}" y="${e.label.box.y.toFixed(1)}" width="${e.label.box.w.toFixed(1)}" height="${e.label.box.h.toFixed(1)}" stroke="#06f" stroke-width="1.5"/>`);
    o.push(`<line x1="${c[0].toFixed(1)}" y1="${c[1].toFixed(1)}" x2="${mid[0].toFixed(1)}" y2="${mid[1].toFixed(1)}" stroke="#06f" stroke-width="1.5" stroke-dasharray="3 3"/>`);
    o.push(`<circle cx="${mid[0].toFixed(1)}" cy="${mid[1].toFixed(1)}" r="4" fill="#06f"/>`);
  }
  o.push('</g>');
  const k = src.lastIndexOf('</svg>');
  return src.slice(0, k) + o.join('\n') + '\n' + src.slice(k);
}

function checkScene(model, st, out) {
  const sc = st.scene;
  if (!sc) { out.push('  no scene in the struct file'); return; }
  out.push(`  canvas: svg ${model.W} x ${model.H}, scene ${sc.width} x ${sc.height} (the SVG prints whole px)`);
  if (sc.nodes.length !== model.nodes.length) { out.push(`  node count differs: svg ${model.nodes.length}, scene ${sc.nodes.length}`); return; }
  // the SVG is the scene translated by (tx, ty): take it from node 0, then compare everything
  const tx = model.nodes[0].box.x - sc.nodes[0].x, ty = model.nodes[0].box.y - sc.nodes[0].y;
  let dn = 0, df = 0, dl = 0, de = 0, deEnd = 0, worst = '', dLenMax = 0, dLenSum = 0, dLenN = 0;
  sc.nodes.forEach((s, i) => {
    const b = model.nodes[i].box;
    const d = Math.max(Math.abs(b.x - s.x - tx), Math.abs(b.y - s.y - ty), Math.abs(b.w - s.w), Math.abs(b.h - s.h));
    if (d > dn) { dn = d; worst = `${s.id} (${st.nodes[i] ? st.nodes[i].shape : '?'})`; }
  });
  if (sc.clusters.length === model.frames.length) sc.clusters.forEach((s, i) => { const b = model.frames[i].box; df = Math.max(df, Math.abs(b.x - s.x - tx), Math.abs(b.y - s.y - ty), Math.abs(b.w - s.w), Math.abs(b.h - s.h)); });
  else out.push(`  frame count differs: svg ${model.frames.length}, scene ${sc.clusters.length}`);
  const drawn = st.edges.map((e, i) => ({ e, s: sc.edges[i] })).filter(x => x.e.drawn !== false);
  let lblN = 0;
  if (drawn.length === model.edges.length) drawn.forEach((x, i) => {
    const e = model.edges[i];
    if (x.s.label && e.label) { lblN++; const c = center(e.label.box); dl = Math.max(dl, Math.abs(c[0] - x.s.label.cx - tx), Math.abs(c[1] - x.s.label.cy - ty), Math.abs(e.label.box.w - x.s.label.w)); }
    else if (!!x.s.label !== !!e.label) out.push(`  label presence differs on edge ${e.name}`);
    // drawn path against the scene's polyline: start points agree, and every scene point lies near the path
    // the drawn path against the scene's waypoints: the stroke stops short under an
    // arrowhead, so points within 16 px of either end are reported apart
    if (x.s.routed && e.segs.length) {
      const P = x.s.points;
      let acc = 0; const at = [0];
      for (let k = 1; k < P.length; k++) { acc += Math.hypot(P[k][0] - P[k - 1][0], P[k][1] - P[k - 1][1]); at.push(acc); }
      P.forEach((p, k) => {
        const d = closestOnSegs(e.segs, [p[0] + tx, p[1] + ty]).d;
        if (at[k] < 16 || acc - at[k] < 16) deEnd = Math.max(deEnd, d); else de = Math.max(de, d);
      });
      const dLen = acc - e.len; // the scene polyline has square corners and reaches the arrow tip
      dLenMax = Math.max(dLenMax, Math.abs(dLen)); dLenSum += dLen; dLenN++;
    }
  });
  else out.push(`  edge count differs: svg ${model.edges.length}, scene drawn ${drawn.length}`);
  out.push(`  translation svg - scene = (${tx.toFixed(2)}, ${ty.toFixed(2)})`);
  out.push(`  max |diff| node boxes ${dn.toFixed(3)} px (${sc.nodes.length} nodes; worst ${worst || '-'}), frames ${df.toFixed(3)} px (${sc.clusters.length}), label chips ${dl.toFixed(3)} px (${lblN})`);
  out.push(`  scene waypoints off the drawn path: ${de.toFixed(3)} px away from the ends, ${deEnd.toFixed(3)} px within 16 px of an end (arrowhead trim)`);
  if (dLenN) out.push(`  length of the scene's waypoint polyline minus the drawn path, ${dLenN} routed edges: mean ${(dLenSum / dLenN).toFixed(2)} px, max |diff| ${dLenMax.toFixed(2)} px`);
}

function analyse(file, opts) {
  const src = fs.readFileSync(file, 'utf8');
  const warns = [];
  const warn = m => warns.push(m);
  const engine = opts.engine || (/aria-roledescription=/.test(src.slice(0, 2000)) ? 'mermaid' : 'fm');
  const doc = parseXml(src);
  const model = engine === 'fm' ? parseFlowmaid(doc, warn) : parseMermaid(doc, warn);
  model.notes = [];
  const base = path.basename(file);
  const name = opts.name || base.replace(/\.svg$/, '').replace(/\.(mermaid|flowmaid|fm\w*|static|routed)$/, '');
  let st = null;
  const stFile = opts.struct || (opts.structDir ? path.join(opts.structDir, name + '.struct.json') : null);
  if (stFile) { if (fs.existsSync(stFile)) st = JSON.parse(fs.readFileSync(stFile, 'utf8')); else warn(`no struct file ${stFile}`); }
  if (model.unknown) warn(`${model.unknown} unrecognised top-level element(s)`);
  resolveStructure(model, st, warn);
  resolveEdges(model, st, warn);
  let dir = opts.dir || (st && st.direction) || null;
  let dirShown = dir;
  if (!dir) { dir = inferDir(model); dirShown = dir + '?'; }
  if (dir === 'TB') dir = 'TD';
  const M = measure(model, dir);
  M.name = name; M.engine = opts.label || engine; M.dir = dirShown;
  M.membership = model.membership; M.edge_ends = model.edgeEnds; M.warn = warns.length;
  return { M, model, st, warns, src, engine };
}

function printDetail(r, out) {
  const { M, model, warns } = r;
  out.push(`== ${M.name} [${M.engine}] ${model.W} x ${model.H}, dir ${M.dir}, membership ${model.membership}, edge ends ${model.edgeEnds}`);
  for (const w of warns) out.push('  WARN ' + w);
  for (const w of model.notes) out.push('  NOTE ' + w);
  out.push(' frames:'); out.push(table(M.detail.frames, ['id', 'parent', 'x', 'y', 'w', 'h', 'members', 'node_area', 'compact', 'span', 'foreign_nodes']).trimEnd());
  out.push(' sibling pairs:'); out.push(table(M.detail.sib, ['a', 'b', 'kind', 'flow_overlap', 'linked']).trimEnd());
  out.push(' edges:'); out.push(table(M.detail.edges, ['edge', 'len', 'straight_max', 'bends', 'detour', 'hops', 'xframe_runs', 'xframe_len', 'node_hits', 'lateral_x', 'flow_x', 'ends']).trimEnd());
  out.push(' crossings of the border of a frame that holds exactly one end:'); out.push(table(M.detail.border, ['edge', 'frame', 'side', 'kind', 'at']).trimEnd());
  out.push(' runs through a frame holding neither end:'); out.push(table(M.detail.xruns, ['edge', 'frame', 'len', 'from', 'to']).trimEnd());
  out.push(' crossings:'); out.push(table(M.detail.crossings, ['a', 'b', 'x', 'y']).trimEnd());
  out.push(' labels:'); out.push(table(M.detail.labels, ['edge', 'text', 'x', 'y', 'w', 'h', 'edge_len', 's', 'off', 'end_px', 'detach', 'own_gap', 'in_end', 'gap', 'on_node', 'struck', 'on_label_after']).trimEnd());
  out.push(` bands: ${M.bands}`);
  out.push(` widest bundle of parallel lanes: ${M.lanes_max} at ${M.lanes_at}`);
}

// ------------------------------------------------------------------ selftest
function selftest() {
  const dir = path.join(__dirname, 'selftest');
  const exp = JSON.parse(fs.readFileSync(path.join(dir, 'expected.json'), 'utf8'));
  let bad = 0, total = 0;
  for (const c of exp.cases) {
    const r = analyse(path.join(dir, c.svg), { struct: c.struct ? path.join(dir, c.struct) : null, dir: c.dir || null });
    for (const [k, want] of Object.entries(c.expect)) {
      total++;
      const got = r.M[k];
      const ok = typeof want === 'number' ? Math.abs(got - want) <= (c.tol && c.tol[k] !== undefined ? c.tol[k] : 1e-3) : got === want;
      if (!ok) { bad++; console.log(`FAIL ${c.svg} ${k}: got ${got}, want ${want}`); }
    }
    if (r.warns.length && !c.allowWarn) { bad++; console.log(`FAIL ${c.svg}: warnings: ${r.warns.join('; ')}`); }
  }
  // unit checks of the geometry kernel
  const unit = (label, got, want, tol) => { total++; if (Math.abs(got - want) > tol) { bad++; console.log(`FAIL unit ${label}: got ${got}, want ${want}`); } };
  unit('quarter arc r=100 length', segsLength(toSegs(flattenPath('M 100 0 A 100 100 0 0 1 0 100', null))), Math.PI * 50, 0.05);
  unit('cubic == straight line length', segsLength(toSegs(flattenPath('M 0 0 C 10 0, 20 0, 30 0', null))), 30, 1e-9);
  unit('quadratic corner 10px ~ arc', segsLength(toSegs(flattenPath('M 0 0 Q 10 0 10 10', null))), 16.2322, 0.01);
  unit('relative path', segsLength(toSegs(flattenPath('M 10 10 h 30 v 40 l -30 -40 z', null))), 30 + 40 + 50, 1e-9);
  const X = (d1, d2) => { const mk = d => { const s = dehop(toSegs(flattenPath(d, null))).segs; return { segs: s, len: segsLength(s) }; }; const a = mk(d1), b = mk(d2); const p = crossingsBetween(a, b), q = crossingsBetween(b, a); return Math.min(p.length, q.length); };
  unit('plain X', X('M 0 0 L 100 100', 'M 0 100 L 100 0'), 1, 0);
  unit('orthogonal +', X('M 50 0 L 50 100', 'M 0 50 L 100 50'), 1, 0);
  unit('T touch is no crossing', X('M 50 0 L 50 50', 'M 0 50 L 100 50'), 0, 0);
  unit('shared trunk, split to both sides', X('M 0 0 L 0 50 L -40 50 L -40 90', 'M 0 0 L 0 50 L 40 50 L 40 90'), 0, 0);
  unit('trunk joined from the left, left to the left', X('M -40 0 L -40 20 L 0 20 L 0 60 L -40 60', 'M 0 0 L 0 100'), 0, 0);
  unit('trunk joined from the left, left to the right', X('M -40 0 L -40 20 L 0 20 L 0 60 L 40 60', 'M 0 0 L 0 100'), 1, 0);
  unit('two crossings (S through a line)', X('M -20 0 L 20 30 L -20 60 L -20 90', 'M 0 -10 L 0 100'), 2, 0);
  unit('hop arc over a line still crosses once', X('M 0 50 L 44 50 A 6 6 0 0 1 56 50 L 100 50', 'M 50 0 L 50 100'), 1, 0);
  unit('parallel 1px apart', X('M 0 0 L 100 0', 'M 0 1 L 100 1'), 0, 0);
  unit('rounded corner crossing', X('M 0 50 L 40 50 Q 50 50 50 60 L 50 100', 'M 0 80 L 100 80'), 1, 0);
  { const d = dehop(toSegs(flattenPath('M 0 50 L 44 50 A 6 6 0 0 1 56 50 L 100 50', null))); unit('dehop count', d.hops, 1, 0); unit('dehop length', segsLength(d.segs), 100, 1e-6); unit('dehop straight', straightRuns(d.segs)[0].len, 100, 1e-6); }
  { const d = dehop(toSegs(flattenPath('M 0 0 L 40 0 L 40 10 L 80 10', null))); unit('a Z jog is not a hop', d.hops, 0, 0); }
  { const d = dehop(toSegs(flattenPath('M 0 50 L 44 50 A 6 6 0 0 1 56 50 A 6 6 0 0 1 68 50 L 100 50', null))); unit('two hops back to back', d.hops, 2, 0); unit('two hops: one straight run', straightRuns(d.segs).length, 1, 0); }
  { const d = dehop(toSegs(flattenPath('M 0 50 L 44 50 A 6 6 0 0 1 56 50 L 56 90', null))); unit('a bump that then turns away is kept', d.hops, 0, 0); }
  { const d = dehop(toSegs(flattenPath('M 0 50 L 44 50 C 46 42, 54 42, 56 50 L 100 50', null))); unit('a cubic bump is a hop too', d.hops, 1, 0); }
  // flowmaid's hops (B11): two quarter circles, and bridges with a level top over 3 and over 6 tracks a pitch (12.5) apart
  { const d = dehop(toSegs(flattenPath('M 0 50 L 45 50 A 5 5 0 0 1 50 45 A 5 5 0 0 1 55 50 L 100 50', null))); unit('fm single hop: count', d.hops, 1, 0); unit('fm single hop: length', segsLength(d.segs), 100, 1e-6); unit('fm single hop: one straight run', straightRuns(d.segs).length, 1, 0); }
  { const d = dehop(toSegs(flattenPath('M 0 50 L 40 50 A 5 5 0 0 1 45 45 L 70 45 A 5 5 0 0 1 75 50 L 120 50', null))); unit('bridge over 3 tracks: count', d.hops, 1, 0); unit('bridge over 3 tracks: length', segsLength(d.segs), 120, 1e-6); unit('bridge over 3 tracks: one straight run', straightRuns(d.segs).length, 1, 0); unit('bridge over 3 tracks: no bend', Math.max(0, straightRuns(d.segs).filter(r => r.len >= BEND_RUN_MIN).length - 1), 0, 0); }
  { const d = dehop(toSegs(flattenPath('M 0 50 L 40 50 A 5 5 0 0 1 45 45 L 107.5 45 A 5 5 0 0 1 112.5 50 L 200 50', null))); unit('bridge over 6 tracks: count', d.hops, 1, 0); unit('bridge over 6 tracks: length', segsLength(d.segs), 200, 1e-6); unit('bridge over 6 tracks: one straight run', straightRuns(d.segs)[0].len, 200, 1e-6); }
  { const d = dehop(toSegs(flattenPath('M 200 50 L 112.5 50 A 5 5 0 0 0 107.5 45 L 45 45 A 5 5 0 0 0 40 50 L 0 50', null))); unit('bridge drawn right to left', d.hops, 1, 0); }
  { const d = dehop(toSegs(flattenPath('M 50 0 L 50 40 A 9 9 0 0 1 59 49 L 59 80 A 9 9 0 0 1 50 89 L 50 150', null))); unit('bridge of the widest radius on a plumb line', d.hops, 1, 0); unit('... its length', segsLength(d.segs), 150, 1e-6); }
  { const d = dehop(toSegs(flattenPath('M 0 50 L 40 50 A 5 5 0 0 1 45 45 L 70 45 A 5 5 0 0 1 75 50 L 90 50 A 5 5 0 0 1 95 45 L 120 45 A 5 5 0 0 1 125 50 L 200 50', null))); unit('two bridges on one leg', d.hops, 2, 0); unit('two bridges: one straight run', straightRuns(d.segs).length, 1, 0); }
  { const d = dehop(toSegs(flattenPath('M 0 50 L 40 50 L 40 45 L 100 45 L 100 50 L 200 50', null))); unit('a square jog up and back is no bridge', d.hops, 0, 0); }
  { const d = dehop(toSegs(flattenPath('M 0 50 L 40 50 A 5 5 0 0 1 45 45 L 70 45 A 5 5 0 0 1 75 50 L 75 90', null))); unit('a bridge that then turns away is kept', d.hops, 0, 0); }
  { const d = dehop(toSegs(flattenPath('M 0 50 L 40 50 A 5 5 0 0 1 45 45 L 70 45 A 5 5 0 0 0 75 40 L 120 40', null))); unit('a level stretch that does not come back is kept', d.hops, 0, 0); }
  { const c = borderCrossings(toSegs([[[0, 5], [100, 5]]]), { x: 20, y: 0, w: 30, h: 10 }); unit('a line through a box crosses its border twice', c.length, 2, 0); unit('... first through the left side', c[0].side === 'L' ? 1 : 0, 1, 0); unit('... then through the right', c[1].side === 'R' ? 1 : 0, 1, 0); }
  { const c = borderCrossings(toSegs([[[30, -10], [30, 5]]]), { x: 20, y: 0, w: 30, h: 10 }); unit('entering through the top', c.length === 1 && c[0].side === 'T' ? 1 : 0, 1, 0); }
  { const c = borderCrossings(toSegs([[[0, 0], [100, 0]]]), { x: 20, y: 0, w: 30, h: 10 }); unit('a run on the border crosses nothing', c.length, 0, 0); }
  { const c = borderCrossings(toSegs([[[0, 5], [20, 5]]]), { x: 20, y: 0, w: 30, h: 10 }); unit('an edge that ends on the border crosses nothing', c.length, 0, 0); }
  { const iv = insideIntervals(toSegs([[[0, 5], [100, 5]]]), { x: 20, y: 0, w: 30, h: 10 }, 0.5); unit('clip run', iv[0][1] - iv[0][0], 29, 1e-9); }
  { const iv = insideIntervals(toSegs([[[0, 0], [100, 0]]]), { x: 20, y: 0, w: 30, h: 10 }, 0.5); unit('border-hugging run is outside', iv.length, 0, 0); }
  console.log(bad ? `selftest: ${bad} of ${total} checks FAILED` : `selftest: all ${total} checks passed`);
  process.exit(bad ? 1 : 0);
}

// ------------------------------------------------------------------ main
function main() {
  const argv = process.argv.slice(2);
  const opts = { files: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--struct') opts.struct = argv[++i];
    else if (a === '--struct-dir') opts.structDir = argv[++i];
    else if (a === '--dir') opts.dir = argv[++i].toUpperCase();
    else if (a === '--engine') opts.engine = argv[++i] === 'mermaid' ? 'mermaid' : 'fm';
    else if (a === '--label') opts.label = argv[++i];
    else if (a === '--name') opts.name = argv[++i];
    else if (a === '--detail') opts.detail = true;
    else if (a === '--overlay') opts.overlay = argv[++i];
    else if (a === '--check-scene') opts.checkScene = true;
    else if (a === '--json') opts.json = true;
    else if (a === '--no-header') opts.noHeader = true;
    else if (a === '--selftest') return selftest();
    else if (a === '--columns') { console.log(COLUMNS.map(c => c[0]).join('\n')); return; }
    else if (a === '-h' || a === '--help') { const ls = fs.readFileSync(__filename, 'utf8').split('\n').slice(2); console.log(ls.slice(0, ls.findIndex(l => !l.startsWith('//'))).map(l => l.slice(3)).join('\n')); return; }
    else if (a.startsWith('--')) { console.error('unknown option ' + a); process.exit(2); }
    else opts.files.push(a);
  }
  if (!opts.files.length) { console.error('usage: blockmetrics [options] file.svg ...   (--help)'); process.exit(2); }
  if (!opts.noHeader && !opts.json) console.log(COLUMNS.map(c => c[0]).join('\t'));
  let failed = 0;
  const jsonOut = [];
  for (const f of opts.files) {
    let r;
    try { r = analyse(f, opts); } catch (e) { console.error(`blockmetrics: ${f}: ${e.message}`); failed++; continue; }
    if (opts.json) jsonOut.push({ metrics: Object.fromEntries(COLUMNS.map(([k]) => [k, r.M[k]])), detail: r.M.detail, warnings: r.warns });
    else console.log(row(r.M));
    const err = [];
    if (opts.detail) printDetail(r, err);
    else for (const w of r.warns) err.push(`blockmetrics: ${r.M.name} [${r.M.engine}]: WARN ${w}`);
    if (opts.checkScene) {
      err.push(`-- check-scene ${r.M.name}`);
      if (r.engine !== 'fm' || !r.st) err.push('  needs a flowmaid SVG and --struct'); else checkScene(r.model, r.st, err);
    }
    if (err.length) console.error(err.join('\n'));
    if (opts.overlay) {
      fs.mkdirSync(opts.overlay, { recursive: true });
      fs.writeFileSync(path.join(opts.overlay, `${r.M.name}.${r.engine}.overlay.svg`), overlaySvg(r.src, r.model, r.M));
    }
  }
  if (opts.json) console.log(JSON.stringify(jsonOut, null, 1));
  process.exit(failed ? 1 : 0);
}
main();
