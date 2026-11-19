// node crop.js <svg> <out.png> x y w h [scale]
const { chromium } = require("playwright"); const fs = require("fs");
(async () => {
  const [svg, out, x, y, w, h, sc] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: Number(sc || 2), viewport: {width: Number(w), height: Number(h)} });
  await p.setContent(`<html><body style="margin:0;background:#fff"><div style="position:absolute;left:${-x}px;top:${-y}px">${fs.readFileSync(svg, "utf8")}</div></body></html>`);
  await p.screenshot({ path: out, clip: {x:0,y:0,width:Number(w),height:Number(h)} }); await b.close();
})();
