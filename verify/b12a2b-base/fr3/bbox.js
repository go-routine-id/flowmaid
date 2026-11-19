const { chromium } = require("playwright"); const fs = require("fs");
(async () => { const b = await chromium.launch(); const p = await b.newPage();
 for (const f of process.argv.slice(2)) { await p.setContent(`<html><body style="margin:0">${fs.readFileSync(f,'utf8')}</body></html>`);
  const r = await p.evaluate(() => [...document.querySelectorAll('text[font-weight="bold"]')].map(t => { const bb = t.getBBox(); return `${t.textContent}: x ${bb.x.toFixed(1)}..${(bb.x+bb.width).toFixed(1)} y ${bb.y.toFixed(1)}..${(bb.y+bb.height).toFixed(1)}`; }));
  console.log(f, r.join(' | ')); } await b.close(); })();
