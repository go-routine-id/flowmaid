const { chromium } = require('playwright');
const fs = require('fs');
(async () => {
  const files = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1.3, viewport: { width: 1800, height: 1200 } });
  for (const f of files) {
    const svg = fs.readFileSync(f, 'utf8');
    await p.setContent('<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:4px">' + svg + '</div></body></html>');
    await (await p.$('#w')).screenshot({ path: f.replace(/\.svg$/, '.png') });
  }
  await b.close();
})();
