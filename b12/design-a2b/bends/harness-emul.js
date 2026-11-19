// usage: node harness-emul.js file.svg ... : emulate the repository harness's bends_max (src/quality.rs
// straight_runs, longest-first, on edge_painted_polyline: each cubic sampled at 12 points, no hops) on the
// painted SVG paths, to explain why it reads differently from blockmetrics. Hop arcs (pairs of `A`, with an
// optional level `L` between) are replaced by their chord, as the harness's polyline has none.
const b = require(__dirname + '/fixed/blockmetrics');
const fs = require('fs');
const N = +(process.env.N || 12);
function flatten12(d) {
  d = d.replace(/A [\d.]+ [\d.]+ 0 0 [01] [\d.-]+ [\d.-]+ (?:L [\d.-]+ [\d.-]+ )?A [\d.]+ [\d.]+ 0 0 [01] ([\d.-]+) ([\d.-]+)/g, 'L $1 $2');
  const tok = d.match(/[MLC]|[-\d.]+/g); const pts = []; let i = 0, cmd = '', x = 0, y = 0;
  const num = () => parseFloat(tok[i++]);
  while (i < tok.length) {
    if (/[MLC]/.test(tok[i])) cmd = tok[i++];
    if (cmd === 'M' || cmd === 'L') { x = num(); y = num(); pts.push([x, y]); }
    else if (cmd === 'C') { const x1 = num(), y1 = num(), x2 = num(), y2 = num(), X = num(), Y = num();
      for (let k = 1; k <= N; k++) { const t = k / N, u = 1 - t; pts.push([u*u*u*x + 3*u*u*t*x1 + 3*u*t*t*x2 + t*t*t*X, u*u*u*y + 3*u*u*t*y1 + 3*u*t*t*y2 + t*t*t*Y]); }
      x = X; y = Y; }
  }
  return pts;
}
for (const f of process.argv.slice(2)) {
  const s = fs.readFileSync(f, 'utf8');
  const re = /<path d="([^"]*)"[^>]*><title>([^<]*)/g; let m, worst = [0, ''];
  const per = [];
  while ((m = re.exec(s))) {
    if (/[QSA]/.test(m[1].replace(/A [\d.]+ [\d.]+ 0 0 [01] [\d.-]+ [\d.-]+ (?:L [\d.-]+ [\d.-]+ )?A [\d.]+ [\d.]+ 0 0 [01] [\d.-]+ [\d.-]+/g, ''))) continue;
    const segs = b.toSegs([flatten12(m[1])]);
    const n1 = Math.max(0, b.seedRuns(segs).filter(r => r.len >= b.BEND_RUN_MIN).length - 1);
    const n3 = Math.max(0, b.turnRuns(segs).length - 1);
    per.push([m[2].split(':')[0], n1, n3]);
    if (n1 > worst[0]) worst = [n1, m[2].split(':')[0]];
  }
  const w3 = Math.max(...per.map(p => p[2]));
  console.log(`${f.split('/').pop()}\tharness rule on 12-point cubics: bends_max ${worst[0]} (${worst[1]})\twith stages 2-3: ${w3}`);
}
