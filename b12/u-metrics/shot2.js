// rasterise an SVG (any viewBox origin) at a given width: node shot2.js in.svg out.png [width] [clip x y w h in svg units]
const { chromium } = require('playwright');
const fs = require('fs');
(async () => {
  const [,, svgPath, pngPath, wArg, cx, cy, cw, ch] = process.argv;
  let svg = fs.readFileSync(svgPath, 'utf8');
  const m = /viewBox="([-\d.]+) ([-\d.]+) ([\d.]+) ([\d.]+)"/.exec(svg);
  let [vx, vy, vw, vh] = [+m[1], +m[2], +m[3], +m[4]];
  if (cw) { vx = +cx; vy = +cy; vw = +cw; vh = +ch; svg = svg.replace(/viewBox="[^"]*"/, `viewBox="${vx} ${vy} ${vw} ${vh}"`); }
  const W = Math.min(+(wArg || 2000), 2400), H = Math.ceil(W * vh / vw);
  svg = svg.replace(/<svg ([^>]*?)width="[^"]*"/, '<svg $1').replace(/style="max-width:[^"]*"/, '').replace('<svg ', `<svg width="${W}" height="${H}" `);
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: W, height: Math.min(H, 4000) }, deviceScaleFactor: 1 });
  await p.setContent(`<html><body style="margin:0;background:#fff">${svg}</body></html>`);
  await p.screenshot({ path: pngPath, fullPage: true });
  await b.close();
})();
