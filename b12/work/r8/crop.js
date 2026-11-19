// node crop.js <in.svg> <out.png> x y w h [scale]  -- crop an SVG to a viewBox region
const { chromium } = require("playwright"); const fs = require("fs");
(async () => {
  const [inp, out, x, y, w, h, s] = process.argv.slice(2);
  const scale = Number(s || 2);
  let svg = fs.readFileSync(inp, "utf8");
  svg = svg.replace(/<svg([^>]*)>/, (m, a) => {
    a = a.replace(/\sviewBox="[^"]*"/, "").replace(/\swidth="[^"]*"/, "").replace(/\sheight="[^"]*"/, "");
    return `<svg${a} viewBox="${x} ${y} ${w} ${h}" width="${w*scale}" height="${h*scale}">`;
  });
  const b = await chromium.launch(); const p = await b.newPage();
  await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block">${svg}</div></body></html>`);
  await (await p.$("#w")).screenshot({ path: out }); await b.close();
})();
