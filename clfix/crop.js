// node crop.js <svg> <png> <x> <y> <w> <h> [scale]
const { chromium } = require("playwright"); const fs = require("fs");
(async () => {
  const [svg, png, x, y, w, h, sc] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: +(sc || 1.5) });
  await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block">${fs.readFileSync(svg, "utf8")}</div></body></html>`);
  await p.screenshot({ path: png, clip: { x: +x, y: +y, width: +w, height: +h }, fullPage: true });
  await b.close();
})();
