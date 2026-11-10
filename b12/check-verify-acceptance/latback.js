'use strict';
// approximate the harness's lat_backtrack on the blockmetrics model: per edge, half the sideways travel beyond the net sideways shift
const fs = require('fs'), path = require('path'), vm = require('vm');
const bm = process.argv[2];
let src = fs.readFileSync(bm, 'utf8').replace(/^#!.*\n/, '').replace(/\nmain\(\);\s*$/, '\nmodule.exports = { analyse };\n');
const mod = { exports: {} };
vm.runInThisContext('(function(require, module, exports, __filename, __dirname){' + src + '\n})')(require, mod, mod.exports, bm, path.dirname(bm));
for (const f of process.argv.slice(4)) {
  const r = mod.exports.analyse(f, { structDir: process.argv[3] });
  const dir = (r.M.dir || 'TD').replace('?', ''); const vertical = dir === 'TD' || dir === 'BT' || dir === 'TB';
  let tot = 0, n = 0; const rows = [];
  for (const e of r.model.edges) {
    if (!e.segs.length) continue;
    let travel = 0;
    for (const g of e.segs) travel += Math.abs(vertical ? g.bx - g.ax : g.by - g.ay);
    const a = e.segs[0], b = e.segs[e.segs.length - 1];
    const net = Math.abs(vertical ? b.bx - a.ax : b.by - a.ay);
    const lb = (travel - net) / 2;
    if (lb > 4) { tot += lb; n++; rows.push([e.name, Math.round(lb)]); }
  }
  rows.sort((x, y) => y[1] - x[1]);
  console.log(`${r.M.name}/${r.M.engine}\tlat_backtrack~ ${Math.round(tot)} px on ${n} edges\ttop: ${rows.slice(0, 4).map(x => x.join(' ')).join(', ')}`);
}
