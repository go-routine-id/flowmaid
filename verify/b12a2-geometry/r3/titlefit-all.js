// usage: node titlefit.js <svg...> : per cluster title, rendered text right vs its frame's right border
const { chromium } = require('playwright'); const fs = require('fs'); const path = require('path');
(async () => {
  const files = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage();
  for (const f of files) {
    await p.setContent(`<html><body style="margin:0">${fs.readFileSync(f, 'utf8')}</body></html>`);
    const res = await p.evaluate(() => {
      const rects = [...document.querySelectorAll('rect[rx="8"]')].map(r => ({ x: +r.getAttribute('x'), y: +r.getAttribute('y'), w: +r.getAttribute('width'), h: +r.getAttribute('height') }));
      const out = [];
      for (const t of document.querySelectorAll('text[font-weight="bold"]')) {
        const bb = t.getBBox(); const tx = +t.getAttribute('x'), ty = +t.getAttribute('y');
        // the frame whose top-left inset holds this title
        let best = null, bd = 1e9;
        for (const r of rects) { const d = Math.abs(r.x + 10 - tx) + Math.abs(ty - r.y - 17); if (tx >= r.x && tx <= r.x + r.w && ty >= r.y && ty <= r.y + 40 && d < bd) { bd = d; best = r; } }
        if (!best) continue;
        out.push({ title: t.textContent.slice(0, 40), over: +(bb.x + bb.width - (best.x + best.w)).toFixed(1), slack: +((best.x + best.w) - (bb.x + bb.width)).toFixed(1) });
      }
      return out;
    });
    for (const r of res) console.log(`${path.basename(f)}\t${r.title}\tover=${r.over}`);
  }
  await b.close();
})();
