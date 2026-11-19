// node png.js out.png in.svg [maxw]
const { chromium } = require('playwright'); const fs = require('fs');
(async () => { const [o, f, mw] = process.argv.slice(2); const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1, viewport: { width: 1800, height: 800 } });
await p.setContent(`<html><body style="margin:0;background:#fff;display:inline-block"><div id=w style="display:inline-block;padding:8px">${fs.readFileSync(f, 'utf8')}</div></body></html>`);
await p.evaluate(() => { const s = document.querySelector('#w svg'); const vb = s.viewBox.baseVal; s.setAttribute('width', vb.width); s.setAttribute('height', vb.height); s.style.maxWidth = 'none'; });
await (await p.$('#w')).screenshot({ path: o }); await b.close(); })();
