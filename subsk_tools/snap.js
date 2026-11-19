const { chromium } = require('/opt/node-tools/node_modules/playwright');
const fs = require('fs');
const pairs = process.argv.slice(2);
(async () => {
  const browser = await chromium.launch();
  const p = await browser.newPage({ deviceScaleFactor: 1.5, viewport: { width: 1200, height: 900 } });
  for (let i = 0; i < pairs.length; i += 2) {
    const svg = fs.readFileSync(pairs[i], 'utf8');
    await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block">${svg}</div></body></html>`);
    await (await p.$('#w')).screenshot({ path: pairs[i + 1] });
  }
  await browser.close();
})();
