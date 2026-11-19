// zoom.js <out.png> <scale> <x> <y> <w> <h> <svg>... : crop the same SVG-user-space window from each SVG, side by side with labels
const { chromium } = require("playwright"); const fs = require("fs");
(async () => {
  const [o, sc, x, y, w, h, ...fs_] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: +sc });
  const parts = fs_.map(f => {
    let s = fs.readFileSync(f, "utf8");
    s = s.replace(/<svg([^>]*)>/, (m, a) => `<svg${a.replace(/ (width|height|viewBox)="[^"]*"/g, "")} width="${w}" height="${h}" viewBox="${x} ${y} ${w} ${h}">`);
    return `<div style="background:#fff"><div style="font:11px sans-serif">${f.split("/").slice(-2).join("/")}</div>${s}</div>`;
  });
  await p.setContent(`<html><body style="margin:0;background:#888"><div id=w style="display:inline-flex;gap:6px">${parts.join("")}</div></body></html>`);
  await (await p.$("#w")).screenshot({ path: o }); await b.close();
})();
