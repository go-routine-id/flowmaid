const { chromium } = require('playwright'); const fs = require('fs');
(async () => { const args = process.argv.slice(2); const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1 });
for (let i = 0; i < args.length; i += 2) { await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block">${fs.readFileSync(args[i],'utf8')}</div></body></html>`);
await p.evaluate(() => { const s = document.querySelector('svg'); const vb = s.viewBox.baseVal; s.setAttribute('width', vb.width); s.setAttribute('height', vb.height); s.style.maxWidth='none'; });
await (await p.$('#w')).screenshot({ path: args[i+1] }); }
await b.close(); })();
