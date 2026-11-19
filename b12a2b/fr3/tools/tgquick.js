// tgquick.js <dir>...: as tgall.js (per svg in the first dir, per bold title, nearest edge centreline in each dir),
// but the edge paths are parsed from their d attribute (M/L/C sampled; A by its chord) instead of getPointAtLength.
const { chromium } = require("playwright"); const fs = require("fs"); const path = require("path");
(async () => {
  const dirs = process.argv.slice(2);
  const files = fs.readdirSync(dirs[0]).filter(f => f.endsWith('.svg')).sort();
  const b = await chromium.launch(); const p = await b.newPage();
  const measure = async (file) => {
    if (!fs.existsSync(file)) return [];
    await p.setContent(`<html><body style="margin:0">${fs.readFileSync(file, 'utf8')}</body></html>`);
    return await p.evaluate(() => {
      const svg = document.querySelector('svg');
      const titles = [...svg.querySelectorAll('text[font-weight="bold"]')].map(t => ({ name: t.textContent, bb: t.getBBox() }));
      const edges = [...svg.querySelectorAll('path')].filter(e => e.getAttribute('fill') === 'none' && e.querySelector('title'));
      const pts = e => {
        const tok = e.getAttribute('d').match(/[MLCA]|-?[\d.]+/g); let i = 0, cur = [0, 0], out = [];
        const num = () => +tok[i++];
        while (i < tok.length) {
          const c = tok[i++];
          if (c === 'M' || c === 'L') { const q = [num(), num()]; if (c === 'L') { for (let s = 1; s <= 20; s++) out.push([cur[0] + (q[0] - cur[0]) * s / 20, cur[1] + (q[1] - cur[1]) * s / 20]); } else out.push(q); cur = q; }
          else if (c === 'C') { const a = [num(), num()], b2 = [num(), num()], q = [num(), num()]; for (let s = 1; s <= 20; s++) { const t = s / 20, u = 1 - t; out.push([u*u*u*cur[0] + 3*u*u*t*a[0] + 3*u*t*t*b2[0] + t*t*t*q[0], u*u*u*cur[1] + 3*u*u*t*a[1] + 3*u*t*t*b2[1] + t*t*t*q[1]]); } cur = q; }
          else if (c === 'A') { num(); num(); num(); num(); num(); const q = [num(), num()]; out.push(q); cur = q; }
          else { i++; }
        }
        return out;
      };
      const segs = edges.map(e => ({ p: pts(e), name: e.querySelector('title').textContent }));
      return titles.map(t => {
        let best = { d: 1e9, e: '' };
        const box = (x, y) => Math.hypot(Math.max(t.bb.x - x, 0, x - (t.bb.x + t.bb.width)), Math.max(t.bb.y - y, 0, y - (t.bb.y + t.bb.height)));
        for (const s of segs) for (let k = 0; k + 1 < s.p.length; k++) {
          const [a, c] = [s.p[k], s.p[k + 1]]; const L = Math.hypot(c[0] - a[0], c[1] - a[1]); const n = Math.max(1, Math.ceil(L / 0.5));
          for (let j = 0; j <= n; j++) { const d = box(a[0] + (c[0] - a[0]) * j / n, a[1] + (c[1] - a[1]) * j / n); if (d < best.d) best = { d, e: s.name }; }
        }
        return { name: t.name, d: best.d, e: best.e };
      });
    });
  };
  for (const f of files) {
    const rs = []; for (const d of dirs) rs.push(await measure(path.join(d, f)));
    for (const t of rs[rs.length - 1]) {
      const cols = rs.map(r => { const o = r.find(x => x.name === t.name); return o ? `${o.d.toFixed(2)} ${o.e}` : 'NA'; });
      console.log(`${f}\t${t.name}\t${cols.join('\t')}`);
    }
  }
  await b.close();
})();
