// usage: node spot.js <blockmetrics> <struct-dir> <svg> <edge>...
const path = require('path'), fs = require('fs');
const [bmf, sd, svgf, ...edges] = process.argv.slice(2);
const b = require(bmf);
const name = path.basename(svgf).replace(/\.mermaid\.svg$|\.svg$/, '');
const stf = path.join(sd, name + '.struct.json');
const r = b.analyse(svgf, { struct: fs.existsSync(stf) ? stf : null });
for (const want of edges) {
  const e = r.model.edges.find(x => x.name === want);
  if (!e) { console.log(want, 'NO EDGE'); continue; }
  const runs = b.turnRuns ? b.turnRuns(e.segs) : b.straightRuns(e.segs);
  const counted = runs.filter(x => x.len >= b.BEND_RUN_MIN);
  const bends = Math.max(0, counted.length - 1);
  const live = Math.max(0, b.anchoredRuns(e.segs).filter(x => x.len >= b.BEND_RUN_MIN).length - 1);
  console.log(name, want, "corrected=" + bends, "liveYardstick=" + live, "runs=" + counted.map(x => Math.round(x.len)).join(","));
}
