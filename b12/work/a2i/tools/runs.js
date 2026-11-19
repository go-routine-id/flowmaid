const b = require('./bmlib.js');
const d = process.argv[2];
const s = b.dehop(b.toSegs(b.flattenPath(d, null)));
const runs = b.straightRuns(s.segs).filter(r => r.len >= b.BEND_RUN_MIN);
console.log('runs', runs.length, 'bends', runs.length - 1);
for (const r of runs) console.log(`  (${r.ax.toFixed(1)},${r.ay.toFixed(1)})-(${r.bx.toFixed(1)},${r.by.toFixed(1)}) len ${r.len.toFixed(1)}`);
