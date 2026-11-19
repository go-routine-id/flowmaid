// usage: node pair.js <outdir> <name...> : pre | now (block mode, static) side by side from on-pre/ on-now/ (or dpre/dnow with DIR=d)
const { chromium } = require('playwright'); const fs = require('fs'); const path = require('path');
(async () => { const [o, ...names] = process.argv.slice(2); fs.mkdirSync(o, { recursive: true });
  const pre = process.env.DIR === 'd' ? 'dpre' : 'on-pre', now = process.env.DIR === 'd' ? 'dnow' : 'on-now';
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1 });
  for (const n of names) { const a = fs.readFileSync(path.join(pre, n + '.svg'), 'utf8'), c = fs.readFileSync(path.join(now, n + '.svg'), 'utf8');
    await p.setContent(`<html><body style="margin:0;background:#fff;font:14px sans-serif"><div id=w style="display:inline-flex;gap:24px;padding:8px;align-items:flex-start"><div><b>pre 9c142b3 (blocks on)</b><br>${a}</div><div><b>now HEAD (blocks on)</b><br>${c}</div></div></body></html>`);
    await (await p.$('#w')).screenshot({ path: path.join(o, n + '.pair.png') }); }
  await b.close(); })();
