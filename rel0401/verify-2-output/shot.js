// node shot.js <out.png> <x> <y> <label> <base.svg> <head.svg>
// Side by side, each SVG with a red crosshair at the caller's (x, y) in
// SVG user space, and a blue dot at user (0,0).
const { chromium } = require('playwright');
const fs = require('fs');
(async () => {
  const [out, x, y, label, bs, hs] = process.argv.slice(2);
  const mark = (svg) => svg.replace(/<\/svg>\s*$/, `<g stroke="red" stroke-width="2" fill="none"><circle cx="${x}" cy="${y}" r="9"/><line x1="${x - 16}" y1="${y}" x2="${+x + 16}" y2="${y}"/><line x1="${x}" y1="${y - 16}" x2="${x}" y2="${+y + 16}"/></g><circle cx="0" cy="0" r="4" fill="blue"/></svg>`);
  const html = `<html><body style="margin:0;background:#ddd;font:14px sans-serif"><div style="display:flex;gap:16px;padding:8px;align-items:flex-start">
    <div><div>origin/dev (base): ${label}</div><div style="background:#fff;display:inline-block">${mark(fs.readFileSync(bs, 'utf8'))}</div></div>
    <div><div>HEAD 48e9073: ${label}</div><div style="background:#fff;display:inline-block">${mark(fs.readFileSync(hs, 'utf8'))}</div></div></div></body></html>`;
  const b = await chromium.launch(); const p = await b.newPage({ viewport: { width: 1400, height: 900 } });
  await p.setContent(html); await p.screenshot({ path: out, fullPage: true }); await b.close();
})();
