// usage: node runs2.js <path d> : pass-1 (anchored, the live yardstick) and corrected runs of one painted path
const b = require(__dirname + '/fixed/blockmetrics');
const d = process.argv[2];
const s = b.dehop(b.toSegs(b.flattenPath(d, null))).segs;
for (const [name, f] of [['anchored (live)', b.anchoredRuns], ['corrected', b.turnRuns]]) {
  const runs = f === b.turnRuns ? f(s) : f(s).filter(r => r.len >= b.BEND_RUN_MIN);
  console.log(`${name}: runs >= 8 px ${runs.length}, bends ${Math.max(0, runs.length - 1)}`);
  for (const r of runs) console.log(`  (${r.ax.toFixed(1)},${r.ay.toFixed(1)})-(${r.bx.toFixed(1)},${r.by.toFixed(1)}) len ${r.len.toFixed(1)}`);
}
