// node crop.js <out.png> <svg> <cx> <cy> <x> <y> <label>: crop around (cx,cy) user space, crosshair at (x,y)
const { chromium } = require('playwright'); const fs = require('fs');
(async () => {
  const [out, f, cx, cy, x, y, label] = process.argv.slice(2);
  let svg = fs.readFileSync(f, 'utf8');
  svg = svg.replace(/ width="[^"]*" height="[^"]*" viewBox="[^"]*"/, ` width="800" height="480" viewBox="${cx - 200} ${cy - 120} 400 240"`);
  svg = svg.replace(/<\/svg>\s*$/, `<g stroke="red" stroke-width="1.5" fill="none"><circle cx="${x}" cy="${y}" r="7"/><line x1="${x - 12}" y1="${y}" x2="${+x + 12}" y2="${y}"/><line x1="${x}" y1="${y - 12}" x2="${x}" y2="${+y + 12}"/></g></svg>`);
  const b = await chromium.launch(); const p = await b.newPage({ viewport: { width: 820, height: 520 } });
  await p.setContent(`<html><body style="margin:0;font:13px sans-serif"><div>${label}</div><div style="background:#fff;outline:1px solid #888;display:inline-block">${svg}</div></body></html>`);
  await p.screenshot({ path: out }); await b.close();
})();
