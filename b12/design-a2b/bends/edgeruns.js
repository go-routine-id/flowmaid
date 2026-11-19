// usage: node edgeruns.js <struct-dir> file.svg <edge-name> : live and corrected runs of one edge (as blockmetrics names it)
const b = require(__dirname + '/fixed/blockmetrics');
const fs = require('fs'), path = require('path');
const [sd, f, want] = process.argv.slice(2);
const name = path.basename(f).replace(/\.mermaid\.svg$|\.svg$/, '');
const st = path.join(sd, name + '.struct.json');
const r = b.analyse(f, { struct: fs.existsSync(st) ? st : null });
const e = r.model.edges.find(x => x.name === want);
if (!e) { console.log('no edge', want, r.model.edges.map(x => x.name).join(' ')); process.exit(1); }
if (e.d) console.log('d =', e.d.length > 900 ? e.d.slice(0, 900) + ' ...' : e.d); else console.log('points', e.polys.map(p => p.length).join(','));
const ang = x => (Math.atan2(x.by - x.ay, x.bx - x.ax) * 180 / Math.PI).toFixed(2);
for (const [nm, fn] of [['live', b.anchoredRuns], ['corrected', b.turnRuns]]) {
  const R = fn === b.turnRuns ? fn(e.segs) : fn(e.segs).filter(x => x.len >= b.BEND_RUN_MIN);
  console.log(`${nm}: bends ${Math.max(0, R.length - 1)}`);
  for (const x of R) console.log(`  (${x.ax.toFixed(2)},${x.ay.toFixed(2)})-(${x.bx.toFixed(2)},${x.by.toFixed(2)}) len ${x.len.toFixed(1)} ang ${ang(x)}`);
}
