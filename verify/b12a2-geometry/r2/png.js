// usage: node png.js <outdir> <scale> <svg...>
const { chromium } = require('playwright'); const fs = require('fs'); const path = require('path');
(async () => { const [o, sc, ...f] = process.argv.slice(2); fs.mkdirSync(o, { recursive: true });
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: +sc });
  for (const x of f) { await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:8px">${fs.readFileSync(x, 'utf8')}</div></body></html>`);
    await (await p.$('#w')).screenshot({ path: path.join(o, path.basename(path.dirname(x)) + '-' + path.basename(x).replace(/\.svg$/, '.png')) }); }
  await b.close(); })();
