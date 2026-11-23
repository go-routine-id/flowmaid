// node shoot.js list.json W H DSF  — list.json = [[html, png], ...]
const { chromium } = require('playwright');
(async () => {
  const [list, W, H, dsf] = [JSON.parse(require('fs').readFileSync(process.argv[2],'utf8')), +process.argv[3], +process.argv[4], +(process.argv[5] || 2)];
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: W, height: H }, deviceScaleFactor: dsf });
  for (const [html, png] of list) {
    await page.goto('file://' + html);
    await page.evaluate(() => document.fonts.ready);
    await page.screenshot({ path: png, clip: { x: 0, y: 0, width: W, height: H } });
  }
  await browser.close();
})();
