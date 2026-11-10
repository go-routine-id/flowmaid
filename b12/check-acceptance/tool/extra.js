'use strict';
// extra.js - measures the spec gates on but blockmetrics does not print, computed on the blockmetrics model.
// usage: node extra.js <blockmetrics-path> [--struct-dir D] [--dir X] file.svg ...
const fs = require('fs'), path = require('path'), vm = require('vm');
const bm = process.argv[2];
let src = fs.readFileSync(bm, 'utf8').replace(/^#!.*\n/, '').replace(/\nmain\(\);\s*$/, '\nmodule.exports = { analyse, holders, distSegsBox, closestOnSegs, center, boxOverlap };\n');
const mod = { exports: {} };
vm.runInThisContext('(function(require, module, exports, __filename, __dirname){' + src + '\n})')(require, mod, mod.exports, bm, path.dirname(bm));
const { analyse, holders, distSegsBox, center } = mod.exports;
const args = process.argv.slice(3); const opts = {}; const files = [];
for (let i = 0; i < args.length; i++) { if (args[i] === '--struct-dir') opts.structDir = args[++i]; else if (args[i] === '--dir') opts.dir = args[++i]; else if (args[i] === '--struct') opts.struct = args[++i]; else files.push(args[i]); }
const rows = [];
for (const f of files) {
  const r = analyse(f, opts); const { model, M } = r;
  let dir = (M.dir || 'TD').replace('?', ''); const vertical = dir === 'TD' || dir === 'BT' || dir === 'TB';
  const F = model.frames, N = model.nodes, E = model.edges;
  const ins = (b, p) => p[0] > b.x && p[0] < b.x + b.w && p[1] > b.y && p[1] < b.y + b.h;
  let lat = 0, latEdges = 0, flowX = 0, nodeLat = 0, nodeEnds = 0;
  let lblOff = 0, lblOwnGapMax = 0, lblBorder = 0, lblInEnd = 0, lblN = 0;
  for (const e of E) {
    if (!e.segs || !e.segs.length || !e.from || !e.to) continue;
    const hf = holders(model, e.from), ht = holders(model, e.to);
    let hit = false;
    F.forEach((fr, fi) => {
      if (hf.has(fi) === ht.has(fi)) return; // exactly one end held
      const b = fr.box;
      for (const g of e.segs) {
        const a = [g.ax, g.ay], c = [g.bx, g.by];
        if (ins(b, a) !== ins(b, c)) {
          const q = ins(b, a) ? c : a;
          const side = vertical ? ((q[0] <= b.x || q[0] >= b.x + b.w) ? 'lat' : 'flow') : ((q[1] <= b.y || q[1] >= b.y + b.h) ? 'lat' : 'flow');
          if (side === 'lat') { lat++; hit = true; } else flowX++;
        }
      }
    });
    if (hit) latEdges++;
    // node ends on a lateral node side
    const ends = [[e.from, [e.segs[0].ax, e.segs[0].ay]], [e.to, [e.segs[e.segs.length - 1].bx, e.segs[e.segs.length - 1].by]]];
    for (const [end, p] of ends) {
      if (end.n === undefined) continue;
      const b = N[end.n].box; nodeEnds++;
      const dl = Math.abs(p[0] - b.x), dr = Math.abs(p[0] - (b.x + b.w)), dt = Math.abs(p[1] - b.y), db = Math.abs(p[1] - (b.y + b.h));
      const m = Math.min(dl, dr, dt, db);
      const lateral = vertical ? (m === dl || m === dr) && m < Math.min(dt, db) : (m === dt || m === db) && m < Math.min(dl, dr);
      if (lateral) nodeLat++;
    }
    if (e.label) {
      lblN++;
      const g = distSegsBox(e.segs, e.label.box);
      if (g > lblOwnGapMax) lblOwnGapMax = g;
      if (g > 1) lblOff++;
      const lb = e.label.box, c = center(lb);
      let onB = false, inEnd = false;
      F.forEach((fr, fi) => {
        const b = fr.box;
        const ox = Math.min(lb.x + lb.w, b.x + b.w) - Math.max(lb.x, b.x), oy = Math.min(lb.y + lb.h, b.y + b.h) - Math.max(lb.y, b.y);
        const overlaps = ox > 0.5 && oy > 0.5;
        const contained = lb.x >= b.x - 0.5 && lb.y >= b.y - 0.5 && lb.x + lb.w <= b.x + b.w + 0.5 && lb.y + lb.h <= b.y + b.h + 0.5;
        if (overlaps && !contained) onB = true;
        if ((hf.has(fi) !== ht.has(fi)) && ins(b, c)) inEnd = true;
      });
      if (onB) lblBorder++;
      if (inEnd) lblInEnd++;
    }
  }
  rows.push({ name: M.name, engine: M.engine, dir, lateral_x: lat, lateral_edges: latEdges, flow_x: flowX, node_ends: nodeEnds, node_lat_ends: nodeLat, lbl_n: lblN, lbl_off_own_edge: lblOff, lbl_own_gap_max: +lblOwnGapMax.toFixed(1), lbl_on_frame_border: lblBorder, lbl_centre_in_end_block: lblInEnd, crossings: M.crossings, warn: r.warns.length });
}
const cols = Object.keys(rows[0]);
console.log(cols.join('\t')); for (const r of rows) console.log(cols.map(c => r[c]).join('\t'));
