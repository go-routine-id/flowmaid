// titlegap.js <dirA> <dirB> : for each svg in both dirs, min distance (px) between each frame title's painted
// text box (bold, as rendered) and every edge stroke centreline; prints per file the smallest gap in A and B.
const { chromium } = require("playwright"); const fs = require("fs"); const path = require("path");
(async () => {
  const [A, B] = process.argv.slice(2);
  const files = fs.readdirSync(B).filter(f => f.endsWith('.svg'));
  const b = await chromium.launch(); const p = await b.newPage();
  const measure = async (file) => {
    await p.setContent(`<html><body style="margin:0">${fs.readFileSync(file, 'utf8')}</body></html>`);
    return await p.evaluate(() => {
      const svg = document.querySelector('svg');
      const titles = [...svg.querySelectorAll('text[font-weight="bold"]')].map(t => ({ name: t.textContent, bb: t.getBBox() }));
      const edges = [...svg.querySelectorAll('path')].filter(e => e.getAttribute('fill') === 'none' && e.querySelector('title'));
      const out = [];
      const samples = edges.map(e => { const L = e.getTotalLength(); const pts = []; for (let s = 0; s <= L; s += 0.5) pts.push(e.getPointAtLength(s)); return { pts, n: e.querySelector("title").textContent }; });
      for (const t of titles) {
        let best = { d: 1e9, e: '' };
        for (const e of samples) {
          
          for (const q of e.pts) {
            
            const dx = Math.max(t.bb.x - q.x, 0, q.x - (t.bb.x + t.bb.width));
            const dy = Math.max(t.bb.y - q.y, 0, q.y - (t.bb.y + t.bb.height));
            const d = Math.hypot(dx, dy);
            if (d < best.d) best = { d, e: e.n };
          }
        }
        out.push({ name: t.name, d: best.d, e: best.e });
      }
      return out;
    });
  };
  for (const f of files) {
    const a = fs.existsSync(path.join(A, f)) ? await measure(path.join(A, f)) : [];
    const c = await measure(path.join(B, f));
    for (const t of c) {
      const o = a.find(x => x.name === t.name);
      const od = o ? o.d : NaN;
      if (t.d < 4 || (o && o.d < 4)) console.log(`${f}\t${t.name}\tpre ${od.toFixed(2)} ${o ? o.e : ''}\thead ${t.d.toFixed(2)} ${t.e}`);
    }
  }
  await b.close();
})();
