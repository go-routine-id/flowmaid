// usage: node pair.js <outpng> <svgA> <labelA> <svgB> <labelB> [scale]: A | B side by side
const { chromium } = require('playwright'); const fs = require('fs');
(async () => { const [o, a, la, c, lc, sc] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: +(sc || 1) });
  const A = fs.readFileSync(a, 'utf8'), C = fs.readFileSync(c, 'utf8');
  await p.setContent(`<html><body style="margin:0;background:#fff;font:14px sans-serif"><div id=w style="display:inline-flex;gap:24px;padding:8px;align-items:flex-start"><div><b>${la}</b><br>${A}</div><div><b>${lc}</b><br>${C}</div></div></body></html>`);
  await (await p.$('#w')).screenshot({ path: o }); await b.close(); })();
