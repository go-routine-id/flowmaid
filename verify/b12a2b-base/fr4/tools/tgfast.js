// tgall.js <dir>...: per svg in the first dir, per frame title, nearest edge centreline distance in each dir
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
      return titles.map(t => {
        let best = { d: 1e9, e: '' };
        for (const e of edges) { const bb = e.getBBox(); const gx = Math.max(t.bb.x - (bb.x + bb.width), 0, bb.x - (t.bb.x + t.bb.width)); const gy = Math.max(t.bb.y - (bb.y + bb.height), 0, bb.y - (t.bb.y + t.bb.height)); if (Math.hypot(gx, gy) > 12) continue;
          const L = e.getTotalLength();
          for (let s = 0; s <= L; s += 0.5) {
            const q = e.getPointAtLength(s);
            const dx = Math.max(t.bb.x - q.x, 0, q.x - (t.bb.x + t.bb.width));
            const dy = Math.max(t.bb.y - q.y, 0, q.y - (t.bb.y + t.bb.height));
            const d = Math.hypot(dx, dy);
            if (d < best.d) best = { d, e: e.querySelector('title').textContent };
          }
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
