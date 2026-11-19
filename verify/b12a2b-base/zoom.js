// node zoom.js <svg> <out.png> x y w h scale
const { chromium } = require('playwright'); const fs = require('fs');
(async () => {
  const [svgf, out, x, y, w, h, sc] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: parseFloat(sc || '3') });
  await p.setContent(`<html><body style="margin:0;background:#fff">${fs.readFileSync(svgf, 'utf8')}</body></html>`);
  await p.evaluate(() => { const s = document.querySelector('svg'); const vb = s.viewBox.baseVal; s.setAttribute('width', vb.width); s.setAttribute('height', vb.height); });
  const d = await p.evaluate(() => { const s = document.querySelector('svg'); return [Math.ceil(s.viewBox.baseVal.width), Math.ceil(s.viewBox.baseVal.height)]; }); await p.setViewportSize({ width: d[0], height: d[1] }); await p.screenshot({ path: out, clip: { x: +x, y: +y, width: +w, height: +h } });
  await b.close();
})();
