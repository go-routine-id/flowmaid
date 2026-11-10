const { chromium } = require('playwright');
const fs = require('fs');
(async () => {
  const [,, svgPath, pngPath] = process.argv;
  const svg = fs.readFileSync(svgPath, 'utf8');
  const m = /viewBox="0 0 ([\d.]+) ([\d.]+)"/.exec(svg);
  const w = Math.ceil(+m[1]), h = Math.ceil(+m[2]);
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: Math.min(w, 2400), height: Math.min(h, 2400) }, deviceScaleFactor: 1 });
  await p.setContent(`<html><body style="margin:0;background:#fff">${svg.replace('<svg ', `<svg width="${Math.min(w,2400)}" `)}</body></html>`);
  await p.screenshot({ path: pngPath, fullPage: true });
  await b.close();
})();
