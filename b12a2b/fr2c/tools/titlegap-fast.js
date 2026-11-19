// titlegap-fast.js <dirA> <dirB> [limit]: as the verifier's titlegap.js (min distance from each frame title's
// rendered text box to every edge stroke centreline, 0.5 px samples), but only paths whose bbox comes within
// `limit` (default 12) px of the text box are sampled. Prints per file and title: the nearest gap in A and B
// whenever either is under 4 px, and a summary line per file pair with the nearest gap overall.
const { chromium } = require("playwright"); const fs = require("fs"); const path = require("path");
(async () => {
  const [A, B, lim] = process.argv.slice(2);
  const limit = +(lim || 12);
  const files = fs.readdirSync(B).filter(f => f.endsWith('.svg')).sort();
  const b = await chromium.launch(); const p = await b.newPage();
  const measure = async (file) => {
    await p.setContent(`<html><body style="margin:0">${fs.readFileSync(file, 'utf8')}</body></html>`);
    return await p.evaluate((limit) => {
      const svg = document.querySelector('svg');
      const titles = [...svg.querySelectorAll('text[font-weight="bold"]')].map(t => ({ name: t.textContent, bb: t.getBBox() }));
      const edges = [...svg.querySelectorAll('path')].filter(e => e.getAttribute('fill') === 'none' && e.querySelector('title'))
        .map(e => ({ e, bb: e.getBBox(), name: e.querySelector('title').textContent }));
      const out = [];
      for (const t of titles) {
        let best = { d: 1e9, e: '' };
        for (const { e, bb, name } of edges) {
          const gx = Math.max(t.bb.x - (bb.x + bb.width), 0, bb.x - (t.bb.x + t.bb.width));
          const gy = Math.max(t.bb.y - (bb.y + bb.height), 0, bb.y - (t.bb.y + t.bb.height));
          if (Math.hypot(gx, gy) > limit) continue;
          const L = e.getTotalLength();
          for (let s = 0; s <= L; s += 0.5) {
            const q = e.getPointAtLength(s);
            const dx = Math.max(t.bb.x - q.x, 0, q.x - (t.bb.x + t.bb.width));
            if (dx > limit) continue;
            const dy = Math.max(t.bb.y - q.y, 0, q.y - (t.bb.y + t.bb.height));
            const d = Math.hypot(dx, dy);
            if (d < best.d) best = { d, e: name };
          }
        }
        out.push({ name: t.name, d: best.d, e: best.e });
      }
      return out;
    }, limit);
  };
  for (const f of files) {
    const a = fs.existsSync(path.join(A, f)) ? await measure(path.join(A, f)) : [];
    const c = await measure(path.join(B, f));
    for (const t of c) {
      const o = a.find(x => x.name === t.name);
      const od = o ? o.d : NaN;
      if (t.d < 4 || (o && o.d < 4)) console.log(`${f}\t${t.name}\tA ${od.toFixed(2)} ${o ? o.e : ''}\tB ${t.d.toFixed(2)} ${t.e}`);
    }
  }
  await b.close();
})();
