const { chromium } = require('playwright');
(async () => { const b = await chromium.launch(); const p = await b.newPage({ viewport: { width: 900, height: 420 }, deviceScaleFactor: 2 });
  for (const t of ['light','dark']) { await p.goto('file://' + process.argv[2] + `/hero.${t}.html`); await p.waitForTimeout(400);
    await p.screenshot({ path: process.argv[2] + `/hero-${t}.png`, omitBackground: true, clip: { x: 0, y: 0, width: 900, height: 420 } }); }
  await b.close(); })();
