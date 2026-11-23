const { chromium } = require('playwright');
(async () => {
  const browser = await chromium.launch();
  const ctx = await browser.newContext({ viewport: { width: 1100, height: 760 }, deviceScaleFactor: 1.5, colorScheme: process.argv[2] || 'light' });
  const page = await ctx.newPage();
  page.on('console', m => console.log('console:', m.text()));
  page.on('requestfailed', r => console.log('failed:', r.url().slice(0,80), r.failure()?.errorText));
  await page.goto('http://127.0.0.1:8731/', { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(() => document.getElementById('version').textContent.includes('engine'));
  await page.waitForTimeout(1500);
  console.log(await page.evaluate(() => document.getElementById('version').textContent));
  console.log(await page.evaluate(() => [...document.fonts].filter(f=>f.status==='loaded').map(f => f.family + ' ' + f.weight).join(', ')));
  await page.screenshot({ path: 'probe-' + (process.argv[2]||'light') + '.png' });
  await browser.close();
})();
