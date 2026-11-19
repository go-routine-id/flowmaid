const { chromium } = require('playwright'); const fs = require('fs');
(async () => {
  const [svg, out, x, y, w, h, scale] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: +scale });
  await p.setContent(`<html><body style="margin:0;background:#fff">${fs.readFileSync(svg, 'utf8')}</body></html>`);
  await p.screenshot({ path: out, clip: { x: +x, y: +y, width: +w, height: +h } });
  await b.close();
})();
