'use strict';
const fs = require('fs'), path = require('path'), vm = require('vm');
const bm = process.argv[2];
let src = fs.readFileSync(bm, 'utf8').replace(/^#!.*\n/, '').replace(/\nmain\(\);\s*$/, '\nmodule.exports = { analyse, holders };\n');
const mod = { exports: {} };
vm.runInThisContext('(function(require, module, exports, __filename, __dirname){' + src + '\n})')(require, mod, mod.exports, bm, path.dirname(bm));
const { analyse, holders } = mod.exports;
for (const f of process.argv.slice(4)) {
  const r = analyse(f, { structDir: process.argv[3] }); const { model, M } = r;
  const dir = (M.dir || 'TD').replace('?', ''); const vertical = dir === 'TD' || dir === 'BT' || dir === 'TB';
  const ins = (b, p) => p[0] > b.x && p[0] < b.x + b.w && p[1] > b.y && p[1] < b.y + b.h;
  for (const e of model.edges) {
    if (!e.segs.length || !e.from || !e.to) continue;
    const hf = holders(model, e.from), ht = holders(model, e.to);
    model.frames.forEach((fr, fi) => {
      if (hf.has(fi) === ht.has(fi)) return; const b = fr.box;
      for (const g of e.segs) { const a = [g.ax, g.ay], c = [g.bx, g.by];
        if (ins(b, a) !== ins(b, c)) { const q = ins(b, a) ? c : a;
          const lat = vertical ? (q[0] <= b.x || q[0] >= b.x + b.w) : (q[1] <= b.y || q[1] >= b.y + b.h);
          if (lat) console.log(`${M.name}/${M.engine} ${dir}\t${e.name}\tframe ${fr.id || fi} box x ${b.x.toFixed(0)}..${(b.x + b.w).toFixed(0)} y ${b.y.toFixed(0)}..${(b.y + b.h).toFixed(0)}\tcrosses at (${q[0].toFixed(1)}, ${q[1].toFixed(1)})\tseg (${a.map(v => v.toFixed(1))}) -> (${c.map(v => v.toFixed(1))})`);
        } }
    });
  }
}
