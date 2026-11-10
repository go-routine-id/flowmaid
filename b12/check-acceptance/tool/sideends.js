'use strict';
const fs = require('fs'), path = require('path'), vm = require('vm');
const bm = process.argv[2];
let src = fs.readFileSync(bm, 'utf8').replace(/^#!.*\n/, '').replace(/\nmain\(\);\s*$/, '\nmodule.exports = { analyse, holders };\n');
const mod = { exports: {} };
vm.runInThisContext('(function(require, module, exports, __filename, __dirname){' + src + '\n})')(require, mod, mod.exports, bm, path.dirname(bm));
const { analyse, holders } = mod.exports;
const r = analyse(process.argv[4], { structDir: process.argv[3] }); const { model, M } = r;
const F = model.frames, N = model.nodes;
const ins = (b, p) => p[0] > b.x && p[0] < b.x + b.w && p[1] > b.y && p[1] < b.y + b.h;
for (const e of model.edges) {
  if (!e.segs.length || !e.from || !e.to) continue;
  const hf = holders(model, e.from), ht = holders(model, e.to);
  let lat = 0;
  F.forEach((fr, fi) => { if (hf.has(fi) === ht.has(fi)) return; const b = fr.box; for (const g of e.segs) { const a = [g.ax, g.ay], c = [g.bx, g.by]; if (ins(b, a) !== ins(b, c)) { const q = ins(b, a) ? c : a; if (q[0] <= b.x || q[0] >= b.x + b.w) lat++; } } });
  const border = [...hf].some(f => !ht.has(f)) || [...ht].some(f => !hf.has(f));
  const ends = [['from', e.from, [e.segs[0].ax, e.segs[0].ay]], ['to', e.to, [e.segs[e.segs.length - 1].bx, e.segs[e.segs.length - 1].by]]];
  const out = [];
  for (const [w, end, p] of ends) { if (end.n === undefined) continue; const b = N[end.n].box; const dl = Math.abs(p[0] - b.x), dr = Math.abs(p[0] - (b.x + b.w)), dt = Math.abs(p[1] - b.y), db = Math.abs(p[1] - (b.y + b.h)); const m = Math.min(dl, dr, dt, db); if ((m === dl || m === dr) && m < Math.min(dt, db)) out.push(w + (m === dl ? ':left' : ':right')); }
  if (out.length) console.log(`${e.name}\tborder_edge=${border}\tlateral_frame_x=${lat}\tside_ends=${out.join(',')}`);
}
