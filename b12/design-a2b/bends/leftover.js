// usage: node leftover.js <struct-dir> file.svg ... : every pair of consecutive counted runs (>= 8 px) of the
// corrected counter that turn < 10 deg and stand < 3 px apart: the gap between them along the path, the
// largest distance of the gap's vertices from the line of the first run, the offset of the second run
const b = require(__dirname + '/fixed/blockmetrics');
const fs = require('fs'), path = require('path');
const [sd, ...files] = process.argv.slice(2);
const ang = x => Math.atan2(x.by - x.ay, x.bx - x.ax) * 180 / Math.PI;
const cls = {};
for (const f of files) {
  const name = path.basename(f).replace(/\.mermaid\.svg$|\.svg$/, '');
  const st = path.join(sd, name + '.struct.json');
  const r = b.analyse(f, { struct: fs.existsSync(st) ? st : null });
  for (const e of r.model.edges) {
    if (!e.segs || !e.segs.length) continue;
    const F = b.turnRuns(e.segs);
    for (let i = 1; i < F.length; i++) {
      const p = F[i - 1], q = F[i];
      let d = Math.abs(ang(p) - ang(q)) % 360; if (d > 180) d = 360 - d;
      const L = Math.hypot(p.bx - p.ax, p.by - p.ay), ux = (p.bx - p.ax) / L, uy = (p.by - p.ay) / L;
      const off = pt => Math.abs((pt[0] - p.ax) * uy - (pt[1] - p.ay) * ux);
      const qoff = Math.max(off([q.ax, q.ay]), off([q.bx, q.by]));
      if (!(d < 5)) continue;
      let gap = 0, dev = 0;
      for (let k = p.k1 + 1; k < q.k0; k++) { gap += e.segs[k].len; dev = Math.max(dev, off([e.segs[k].bx, e.segs[k].by])); }
      const hasArc = /[Aa]/.test(e.d || '') && dev > 3 && qoff < 1.5;
      const kind = gap < 1e-9 ? 'kink(no gap)' : dev > 3 && qoff < 1.5 ? (hasArc ? 'hop not bridged' : 'bump>3px') : qoff <= 0.75 ? 'jog<=0.75px' : qoff <= 3 ? 'jog0.75-3px' : 'jog>3px';
      cls[kind] = (cls[kind] || 0) + 1;
      console.log([name, e.name, kind, `gap ${gap.toFixed(1)}`, `dev ${dev.toFixed(2)}`, `qoff ${qoff.toFixed(2)}`, `turn ${d.toFixed(2)}`, `at (${p.bx.toFixed(1)},${p.by.toFixed(1)})`].join('\t'));
    }
  }
}
console.error(JSON.stringify(cls));
