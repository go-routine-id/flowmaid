// usage: node upcases.js <struct-dir> file.svg ... : edges whose corrected count is above the live one, and why:
// each corrected run >= 8 px that no live run >= 8 px overlaps by half its length, with the turns into and out of it
const b = require(__dirname + '/fixed/blockmetrics');
const fs = require('fs'), path = require('path');
const [sd, ...files] = process.argv.slice(2);
const ang = x => Math.atan2(x.by - x.ay, x.bx - x.ax) * 180 / Math.PI;
const turn = (p, q) => { let d = Math.abs(ang(p) - ang(q)) % 360; return d > 180 ? 360 - d : d; };
for (const f of files) {
  const name = path.basename(f).replace(/\.mermaid\.svg$|\.svg$/, '');
  const st = path.join(sd, name + '.struct.json');
  const r = b.analyse(f, { struct: fs.existsSync(st) ? st : null });
  for (const e of r.model.edges) {
    if (!e.segs || !e.segs.length) continue;
    const s0 = k => e.segs[k].s0, s1 = k => e.segs[k].s0 + e.segs[k].len;
    const L = b.anchoredRuns(e.segs).filter(x => x.len >= b.BEND_RUN_MIN);
    const F = b.turnRuns(e.segs);
    if (F.length <= L.length) continue;
    const curveCmds = (e.d || '').replace(/[^CcSsQqAa]/g, '').length;
    const out = [];
    F.forEach((x, i) => {
      const a = s0(x.k0), z = s1(x.k1);
      const cov = Math.max(0, ...L.map(y => Math.min(z, s1(y.k1)) - Math.max(a, s0(y.k0))));
      if (cov >= 0.5 * x.len) return;
      const tin = i > 0 ? turn(F[i - 1], x).toFixed(1) : '-', tout = i + 1 < F.length ? turn(x, F[i + 1]).toFixed(1) : '-';
      out.push(`new run len ${x.len.toFixed(1)} at (${x.ax.toFixed(1)},${x.ay.toFixed(1)})-(${x.bx.toFixed(1)},${x.by.toFixed(1)}) turn in ${tin} out ${tout}`);
    });
    console.log(`${name}\t${e.name}\tlive ${Math.max(0, L.length - 1)} -> ${Math.max(0, F.length - 1)}\tcubics ${(e.d || '').split('C').length - 1}\t${out.join(' | ')}`);
  }
}
