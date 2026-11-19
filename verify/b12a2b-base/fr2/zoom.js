// zoom.js <svg> <out.png> <x0> <y0> <w> <h> <scale>
const { chromium } = require("playwright"); const fs = require("fs");
(async () => {
  const [svg, out, x0, y0, w, h, sc] = process.argv.slice(2);
  let s = fs.readFileSync(svg, 'utf8');
  s = s.replace(/viewBox="[^"]*"/, `viewBox="${x0} ${y0} ${w} ${h}"`).replace(/width="[^"]*"/, `width="${w}"`).replace(/height="[^"]*"/, `height="${h}"`);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: parseFloat(sc) });
  await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block">${s}</div></body></html>`);
  await (await p.$('#w')).screenshot({ path: out }); await b.close();
})();
