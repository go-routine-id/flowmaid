// usage: node sampling.js file.svg ... : the same painted paths (hops replaced by chords, flowmaid L/C only)
// sampled at N = 6, 12, 24 points per cubic and at blockmetrics' own ~1 px: how many edges change their
// count with the sampling, live rule vs corrected rule. A truthful counter does not depend on the sampling.
const b = require(__dirname + '/fixed/blockmetrics');
const fs = require('fs');
const HOP = /A [\d.]+ [\d.]+ 0 0 [01] [\d.-]+ [\d.-]+ (?:L [\d.-]+ [\d.-]+ )?A [\d.]+ [\d.]+ 0 0 [01] ([\d.-]+) ([\d.-]+)/g;
function flat(d, N) {
  const tok = d.match(/[MLC]|[-\d.]+/g); const pts = []; let i = 0, cmd = '', x = 0, y = 0; const num = () => parseFloat(tok[i++]);
  while (i < tok.length) {
    if (/[MLC]/.test(tok[i])) cmd = tok[i++];
    if (cmd === 'M' || cmd === 'L') { x = num(); y = num(); pts.push([x, y]); }
    else { const x1 = num(), y1 = num(), x2 = num(), y2 = num(), X = num(), Y = num();
      const steps = N || Math.max(6, Math.min(160, Math.ceil(Math.hypot(x1 - x, y1 - y) + Math.hypot(x2 - x1, y2 - y1) + Math.hypot(X - x2, Y - y2))));
      for (let k = 1; k <= steps; k++) { const t = k / steps, u = 1 - t; pts.push([u*u*u*x + 3*u*u*t*x1 + 3*u*t*t*x2 + t*t*t*X, u*u*u*y + 3*u*u*t*y1 + 3*u*t*t*y2 + t*t*t*Y]); }
      x = X; y = Y; }
  }
  return pts;
}
const live = s => Math.max(0, b.anchoredRuns(s).filter(r => r.len >= b.BEND_RUN_MIN).length - 1);
const fix = s => Math.max(0, b.turnRuns(s).length - 1);
let edges = 0; const diff = { live: 0, fixed: 0 }; const ex = [];
for (const f of process.argv.slice(2)) {
  const s = fs.readFileSync(f, 'utf8'); const re = /<path d="([^"]*)"[^>]*><title>([^<]*)/g; let m;
  while ((m = re.exec(s))) {
    const d = m[1].replace(HOP, 'L $1 $2');
    if (/[QSAZ]/i.test(d.replace(/[MLC]/g, ''))) continue;
    edges++;
    for (const [nm, fn] of [['live', live], ['fixed', fix]]) {
      const v = [0, 6, 12, 24].map(N => fn(b.toSegs([flat(d, N)])));
      if (new Set(v).size > 1) { diff[nm]++; if (nm === 'fixed' && ex.length < 12) ex.push(`${f.split('/').pop()} ${m[2].split(':')[0]} [1px,6,12,24] = ${v.join(',')}`); }
    }
  }
}
console.log(JSON.stringify({ edges, sampling_dependent: diff }));
for (const e of ex) console.log('  ' + e);
