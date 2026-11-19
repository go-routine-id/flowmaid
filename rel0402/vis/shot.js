const { chromium } = require('playwright');
const fs = require('fs'), path = require('path');
(async () => {
  const dir = process.argv[2]; const pairs = process.argv.slice(3);
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1800, height: 900 } });
  for (const p of pairs) {
    const a = fs.readFileSync(path.join(dir, p + '-partial.svg'), 'utf8');
    const b = fs.readFileSync(path.join(dir, p + '-full.svg'), 'utf8');
    await page.setContent(`<html><body style="margin:0;background:#fff;font:14px sans-serif"><table id=t><tr><th>partial</th><th>full route</th></tr><tr><td style="vertical-align:top">${a}</td><td style="vertical-align:top;border-left:2px solid #ccc">${b}</td></tr></table></body></html>`);
    await (await page.$('#t')).screenshot({ path: path.join(dir, p + '.png') });
  }
  await browser.close();
})();
