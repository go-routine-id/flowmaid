// usage: node crop.js <svg> <out.png> <x> <y> <w> <h> [scale]   (clip in SVG user units)
const { chromium } = require('playwright'); const fs = require('fs');
(async () => {
  const [svgPath, out, x, y, w, h, sc] = process.argv.slice(2);
  const k = parseFloat(sc || '1');
  let svg = fs.readFileSync(svgPath, 'utf8');
  const m = /viewBox="([-\d.]+) ([-\d.]+) ([\d.]+) ([\d.]+)"/.exec(svg);
  const W = +m[3], H = +m[4];
  svg = svg.replace('<svg ', `<svg width="${W * k}" height="${H * k}" `);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1, viewport: { width: 1200, height: 900 } });
  await p.setContent(`<html><body style="margin:0;background:#fff">${svg}</body></html>`);
  await p.screenshot({ path: out, fullPage: true, clip: { x: (+x - +m[1]) * k, y: (+y - +m[2]) * k, width: Math.min(+w, W - +x) * k, height: Math.min(+h, H - +y) * k } });
  await b.close();
})();
