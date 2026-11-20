const { chromium } = require('playwright');
(async () => { const b = await chromium.launch(); const p = await b.newPage({ viewport: { width: 1080, height: 1350 }, deviceScaleFactor: 2 });
  await p.goto('file://' + process.argv[2]); await p.waitForTimeout(500);
  await p.screenshot({ path: process.argv[3], clip: { x: 0, y: 0, width: 1080, height: 1350 } }); await b.close(); })();
