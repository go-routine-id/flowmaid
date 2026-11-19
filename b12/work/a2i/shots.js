// usage: node shots.js <outdir> <svg...>: one png per svg
const { chromium } = require('playwright'); const fs = require('fs'); const path = require('path');
(async () => { const [o, ...svgs] = process.argv.slice(2); fs.mkdirSync(o, { recursive: true });
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: +(process.env.SC || 1) });
  for (const s of svgs) { const a = fs.readFileSync(s, 'utf8');
    await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:4px">${a}</div></body></html>`);
    await (await p.$('#w')).screenshot({ path: path.join(o, path.basename(s, '.svg') + '.png') }); }
  await b.close(); })();
