// crop.js <svg> <out.png> <scale> [x y w h] : render an svg at scale, optional crop in svg px
const { chromium } = require("playwright"); const fs = require("fs");
(async () => {
  const [svg, out, scale, x, y, w, h] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: parseFloat(scale || "2"), viewport: { width: 1600, height: 3200 } });
  await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block">${fs.readFileSync(svg, "utf8")}</div></body></html>`);
  const el = await p.$("#w");
  if (x !== undefined) await p.screenshot({ path: out, clip: { x: +x, y: +y, width: +w, height: +h } });
  else await el.screenshot({ path: out });
  await b.close();
})();
