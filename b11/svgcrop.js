// node svgcrop.js in.svg out.png x y w h [scale]  (x,y,w,h in SVG user units)
const { chromium } = require('playwright'); const fs = require('fs');
(async () => {
  const [inp, out, x, y, w, h, sc] = process.argv.slice(2);
  const s = parseFloat(sc || '3');
  let svg = fs.readFileSync(inp, 'utf8');
  svg = svg.replace(/<svg([^>]*?)\sviewBox="[^"]*"/, `<svg$1 viewBox="${x} ${y} ${w} ${h}"`)
           .replace(/<svg([^>]*?)\swidth="[^"]*"/, `<svg$1 width="${w*s}"`)
           .replace(/<svg([^>]*?)\sheight="[^"]*"/, `<svg$1 height="${h*s}"`);
  const b = await chromium.launch(); const p = await b.newPage({ viewport: { width: Math.ceil(w*s)+20, height: Math.ceil(h*s)+20 } });
  await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block">${svg}</div></body></html>`);
  await (await p.$('#w')).screenshot({ path: out }); await b.close();
})();
