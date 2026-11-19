// usage: node peredge.js <struct-dir> <out.tsv> file.svg ...
// per edge: live (first-segment line) and corrected (longest-segment line) runs >= 8 px, straight_max,
// plus two checks on the corrected runs: consecutive counted runs that turn less than 10 deg (a split
// left over) and runs >= 8 px that lie on a curve (segments spanning more than 3 deg).
const b = require(__dirname + '/fixed/blockmetrics');
const fs = require('fs'), path = require('path');
const [sd, out, ...files] = process.argv.slice(2);
const lines = ['file\tedge\tlive_bends\tfix_bends\tlive_smax\tfix_smax\tleft_split\tcurved_runs\tcurve_cmds'];
const ang = (r) => Math.atan2(r.by - r.ay, r.bx - r.ax) * 180 / Math.PI;
const dang = (p, q) => { let d = Math.abs(ang(p) - ang(q)) % 360; return d > 180 ? 360 - d : d; };
let tot = { edges: 0, up: 0, down: 0, left: 0, curved: 0 };
for (const f of files) {
  const name = path.basename(f).replace(/\.mermaid\.svg$|\.svg$/, '');
  const st = path.join(sd, name + '.struct.json');
  const r = b.analyse(f, { struct: fs.existsSync(st) ? st : null });
  for (const e of r.model.edges) {
    if (!e.segs || !e.segs.length) continue;
    tot.edges++;
    const L = b.anchoredRuns(e.segs).filter(x => x.len >= b.BEND_RUN_MIN);
    const allF = b.straightRuns(e.segs);
    const F = b.turnRuns(e.segs);
    const lb = Math.max(0, L.length - 1), fb = Math.max(0, F.length - 1);
    let left = 0;
    const offLine = (p, q) => { const L = Math.hypot(p.bx - p.ax, p.by - p.ay), ux = (p.bx - p.ax) / L, uy = (p.by - p.ay) / L; return Math.max(Math.abs((q.ax - p.ax) * uy - (q.ay - p.ay) * ux), Math.abs((q.bx - p.ax) * uy - (q.by - p.ay) * ux)); };
    for (let i = 1; i < F.length; i++) if (dang(F[i - 1], F[i]) < 10 && offLine(F[i - 1], F[i]) < 3) left++;
    let curved = 0;
    for (const x of F) { let lo = Infinity, hi = -Infinity; const a0 = ang(x); for (let k = x.k0; k <= x.k1; k++) { const g = e.segs[k]; let d = Math.atan2(g.by - g.ay, g.bx - g.ax) * 180 / Math.PI - a0; d = ((d + 540) % 360) - 180; lo = Math.min(lo, d); hi = Math.max(hi, d); } if (hi - lo > 3.0 + 1e-9) curved++; }
    const smaxL = Math.max(0, ...b.anchoredRuns(e.segs).map(x => x.len)), smaxF = Math.max(0, ...allF.map(x => x.len));
    const cc = (e.d || '').replace(/[^CcSsQqTtAa]/g, '').length;
    if (fb > lb) tot.up++; if (fb < lb) tot.down++; if (left) tot.left++; if (curved) tot.curved++;
    lines.push([name, e.name, lb, fb, smaxL.toFixed(1), smaxF.toFixed(1), left, curved, cc].join('\t'));
  }
}
fs.writeFileSync(out, lines.join('\n') + '\n');
console.log(JSON.stringify(tot));
