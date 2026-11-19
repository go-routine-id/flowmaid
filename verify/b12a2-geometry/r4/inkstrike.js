// usage: node inkstrike.js <svg...> : per bold cluster title, the least distance (px) from any edge stroke
// (path with fill="none" and a marker or stroke, outside <defs>) to the title's rendered ink box.
const { chromium } = require('playwright'); const fs = require('fs'); const path = require('path');
(async () => {
  const files = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage();
  for (const f of files) {
    await p.setContent(`<html><body style="margin:0">${fs.readFileSync(f, 'utf8')}</body></html>`);
    const res = await p.evaluate(() => {
      const out = [];
      const paths = [...document.querySelectorAll('path')].filter(e => !e.closest('defs') && e.getAttribute('fill') === 'none' && e.getAttribute('stroke') && e.getAttribute('stroke') !== 'none');
      const samples = paths.map(e => { const L = e.getTotalLength(); const n = Math.max(2, Math.ceil(L / 0.5)); const pts = []; for (let k = 0; k <= n; k++) { const q = e.getPointAtLength(L * k / n); pts.push([q.x, q.y]); } return { pts, w: +(e.getAttribute('stroke-width') || 1) }; });
      for (const t of document.querySelectorAll('text[font-weight="bold"]')) {
        if (t.parentElement && t.parentElement.tagName === 'text') continue;
        const bb = t.getBBox();
        let best = 1e9;
        for (const s of samples) for (const [x, y] of s.pts) {
          const dx = Math.max(bb.x - x, 0, x - (bb.x + bb.width)); const dy = Math.max(bb.y - y, 0, y - (bb.y + bb.height));
          const d = Math.hypot(dx, dy) - s.w / 2; if (d < best) best = d;
        }
        out.push({ title: t.textContent.slice(0, 40), d: +best.toFixed(1) });
      }
      return out;
    });
    for (const r of res) console.log(`${path.basename(f)}\t${r.title}\tdist=${r.d}`);
  }
  await b.close();
})();
