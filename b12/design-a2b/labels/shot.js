// usage: node shot.js <svg> <png> [scale] [clipx clipy clipw cliph]
const { chromium } = require('playwright'); const fs = require('fs');
(async () => { const [svg, png, sc, cx, cy, cw, ch] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: +(sc || 1) });
  const s = fs.readFileSync(svg, 'utf8');
  await p.setContent(`<html><body style="margin:0;background:#fff">${s}</body></html>`);
  const el = await p.$('svg'); const bb = await el.boundingBox();
  await p.setViewportSize({ width: Math.ceil(bb.width) + 10, height: Math.ceil(bb.height) + 10 });
  const opt = { path: png };
  if (cx !== undefined) opt.clip = { x: +cx, y: +cy, width: +cw, height: +ch };
  else opt.clip = { x: 0, y: 0, width: bb.width, height: bb.height };
  await p.screenshot(opt); await b.close(); })();
