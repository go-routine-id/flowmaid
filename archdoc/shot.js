const { chromium } = require('playwright'); const fs = require('fs'), path = require('path');
(async () => { const [dir, out] = process.argv.slice(2); const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1.5 });
  for (const f of fs.readdirSync(dir).filter(x => x.endsWith('.svg'))) {
    const svg = fs.readFileSync(path.join(dir, f), 'utf8');
    await p.setContent(`<html><body style="margin:0;background:#f9fafb;display:inline-block">${svg}</body></html>`);
    const el = await p.$('svg'); await el.screenshot({ path: path.join(out, f.replace('.svg', '.png')) }); }
  await b.close(); })();
