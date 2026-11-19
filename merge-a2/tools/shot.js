// node shot.js <svg> <out.png> [scale] [x y w h]  (x y w h in SVG user units of the viewBox)
const { chromium } = require('playwright'); const fs = require('fs');
(async () => {
  const [svgf, out, sc, x, y, w, h] = process.argv.slice(2);
  const scale = parseFloat(sc || '1.5');
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: scale });
  await p.setContent(`<html><body style="margin:0;background:#fff">${fs.readFileSync(svgf, 'utf8')}</body></html>`);
  const d = await p.evaluate(() => { const s = document.querySelector('svg'); const vb = s.viewBox.baseVal; s.setAttribute('width', vb.width); s.setAttribute('height', vb.height); return [vb.x, vb.y, Math.ceil(vb.width), Math.ceil(vb.height)]; });
  await p.setViewportSize({ width: d[2], height: d[3] });
  const clip = x !== undefined ? { x: +x - d[0], y: +y - d[1], width: +w, height: +h } : { x: 0, y: 0, width: d[2], height: d[3] };
  await p.screenshot({ path: out, clip });
  await b.close();
})();
