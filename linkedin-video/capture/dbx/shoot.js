// node shoot.js list.json W H DSF  -- list.json = [[html, png], ...]
const { chromium } = require('playwright');
(async () => {
  const list = JSON.parse(require('fs').readFileSync(process.argv[2], 'utf8'));
  const W = +process.argv[3], H = +process.argv[4], dsf = +(process.argv[5] || 2);
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: W, height: H }, deviceScaleFactor: dsf });
  for (const [html, png] of list) {
    await page.goto('file://' + html);
    await page.evaluate(() => document.fonts.ready);
    await page.screenshot({ path: png, clip: { x: 0, y: 0, width: W, height: H } });
  }
  await browser.close();
})();
