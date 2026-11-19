// node croppair.js <out.png> <svgA> <labelA> <svgB> <labelB> x y w h [scale]
const { chromium } = require('playwright'); const fs = require('fs');
(async () => { const [o, a, la, b, lb, x, y, w, h, sc] = process.argv.slice(2);
  const br = await chromium.launch(); const p = await br.newPage({ deviceScaleFactor: +(sc || 1) });
  const clip = (s) => `<div style="width:${w}px;height:${h}px;overflow:hidden;position:relative;border:1px solid #ccc"><div style="position:absolute;left:${-x}px;top:${-y}px">${fs.readFileSync(s,'utf8')}</div></div>`;
  await p.setContent(`<html><body style="margin:0;background:#fff;font:14px sans-serif"><div id=w style="display:inline-flex;gap:16px;padding:8px;align-items:flex-start"><div><b>${la}</b><br>${clip(a)}</div><div><b>${lb}</b><br>${clip(b)}</div></div></body></html>`);
  await (await p.$('#w')).screenshot({ path: o }); await br.close(); })();
