// usage: node turnhist.js <struct-dir> file.svg ... : what each counted bend turns by (the angle between
// consecutive straight runs >= 8 px), live counter vs corrected; a bend of < 5 deg is not an axis turn
const b = require(__dirname + '/fixed/blockmetrics');
const fs = require('fs'), path = require('path');
const [sd, ...files] = process.argv.slice(2);
const ang = x => Math.atan2(x.by - x.ay, x.bx - x.ax) * 180 / Math.PI;
const turn = (p, q) => { let d = Math.abs(ang(p) - ang(q)) % 360; return d > 180 ? 360 - d : d; };
const bins = [[0, 5], [5, 45], [45, 135], [135, 181]];
const H = { live: bins.map(() => 0), corrected: bins.map(() => 0) };
for (const f of files) {
  const name = path.basename(f).replace(/\.mermaid\.svg$|\.svg$/, '');
  const st = path.join(sd, name + '.struct.json');
  const r = b.analyse(f, { struct: fs.existsSync(st) ? st : null });
  for (const e of r.model.edges) {
    if (!e.segs || !e.segs.length) continue;
    for (const [nm, fn] of [['live', b.anchoredRuns], ['corrected', b.turnRuns]]) {
      const R = fn === b.turnRuns ? fn(e.segs) : fn(e.segs).filter(x => x.len >= b.BEND_RUN_MIN);
      for (let i = 1; i < R.length; i++) { const t = turn(R[i - 1], R[i]); H[nm][bins.findIndex(([lo, hi]) => t >= lo && t < hi)]++; }
    }
  }
}
for (const nm of ['live', 'corrected']) console.log(nm.padEnd(10), bins.map(([lo, hi], i) => `${lo}-${hi === 181 ? 180 : hi} deg: ${H[nm][i]}`).join('  '), ' total', H[nm].reduce((a, c) => a + c, 0));
