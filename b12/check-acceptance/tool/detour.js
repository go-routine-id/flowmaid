'use strict';
const fs = require('fs'), path = require('path'), vm = require('vm');
const bm = process.argv[2];
let src = fs.readFileSync(bm, 'utf8').replace(/^#!.*\n/, '').replace(/\nmain\(\);\s*$/, '\nmodule.exports = { analyse };\n');
const mod = { exports: {} };
vm.runInThisContext('(function(require, module, exports, __filename, __dirname){' + src + '\n})')(require, mod, mod.exports, bm, path.dirname(bm));
for (const f of process.argv.slice(4)) {
  const r = mod.exports.analyse(f, { structDir: process.argv[3] });
  const rows = [];
  for (const e of r.model.edges) { if (!e.segs.length) continue; const a = e.segs[0], b = e.segs[e.segs.length - 1]; const man = Math.abs(b.bx - a.ax) + Math.abs(b.by - a.ay); rows.push({ n: e.name, len: e.len, ratio: man > 1 ? e.len / man : 1 }); }
  rows.sort((x, y) => y.ratio - x.ratio);
  const over = rows.filter(x => x.ratio > 1.5).length;
  console.log(`${r.M.name}/${r.M.engine}\tmax_len ${Math.round(Math.max(...rows.map(x => x.len)))}\tdetour>1.5x: ${over}\tworst: ${rows.slice(0, 3).map(x => `${x.n} ${x.ratio.toFixed(2)}x (${Math.round(x.len)} px)`).join(', ')}`);
}
