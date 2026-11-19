// node zoom.js <svg> <out.png> x y w h [scale]: crop of an SVG in its own coordinates
const { chromium } = require("playwright"); const fs = require("fs");
(async () => {
  const [svg, out, x, y, w, h, scale] = process.argv.slice(2);
  const src = fs.readFileSync(svg, "utf8");
  const m = src.match(/width="([\d.]+)" height="([\d.]+)"/);
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: Math.ceil(+m[1]) + 10, height: Math.ceil(+m[2]) + 10 }, deviceScaleFactor: +(scale || 2) });
  await p.setContent(`<html><body style="margin:0;background:#fff">${src}</body></html>`);
  await p.screenshot({ path: out, clip: { x: +x, y: +y, width: +w, height: +h } }); await b.close();
})();
