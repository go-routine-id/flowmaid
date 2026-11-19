const { chromium } = require('playwright');
const fs = require('fs'), path = require('path');
(async () => {
  const dir = process.argv[2]; const names = process.argv.slice(3);
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, deviceScaleFactor: 1 });
  for (const n of names) {
    await page.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block">${fs.readFileSync(path.join(dir, n + '.svg'), 'utf8')}</div></body></html>`);
    await page.evaluate(() => { const s = document.querySelector('#w svg'); const vb = s.viewBox.baseVal; s.setAttribute('width', vb.width); s.setAttribute('height', vb.height); s.style.maxWidth='none'; });
    await (await page.$('#w')).screenshot({ path: path.join(dir, n + '.png') });
  }
  await browser.close();
})();
