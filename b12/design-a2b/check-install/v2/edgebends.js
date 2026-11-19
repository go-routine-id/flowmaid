// usage: node edgebends.js <blockmetrics> <svg> <struct-dir> <edge>...
const [bm, svgf, sd, ...want] = process.argv.slice(2);
const b = require(bm), fs = require('fs'), path = require('path');
const name = path.basename(svgf).replace(/\.mermaid\.svg$|\.svg$/, '');
const stf = path.join(sd, name + '.struct.json');
const r = b.analyse(svgf, { struct: fs.existsSync(stf) ? stf : null });
for (const w of want) {
  const e = r.model.edges.find(x => x.name === w);
  if (!e) { console.log(w, 'NOT FOUND'); continue; }
  const tr = b.turnRuns(e.segs);
  const sr = b.straightRuns(e.segs).filter(x => x.len >= b.BEND_RUN_MIN);
  const ar = b.anchoredRuns(e.segs).filter(x => x.len >= b.BEND_RUN_MIN);
  console.log(w, 'new bends', Math.max(0, tr.length - 1), '| straightRuns>=8:', sr.length, '| old (anchored) bends', Math.max(0, ar.length - 1));
  console.log('  runs:', tr.map(x => `(${x.ax.toFixed(1)},${x.ay.toFixed(1)})->(${x.bx.toFixed(1)},${x.by.toFixed(1)}) ${x.len.toFixed(1)}`).join('  '));
}
