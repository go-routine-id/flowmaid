// usage: node zoom.js <svg> <out.png> x y w h scale
const { chromium } = require('playwright'); const fs = require('fs');
(async () => { const [f, o, x, y, w, h, sc] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: +sc, viewport: { width: 4000, height: 4000 } });
  await p.setContent(`<html><body style="margin:0;background:#fff">${fs.readFileSync(f, 'utf8')}</body></html>`);
  await p.screenshot({ path: o, clip: { x: +x, y: +y, width: +w, height: +h } }); await b.close(); })();
