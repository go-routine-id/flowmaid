const { chromium } = require('playwright');
(async () => {
  const b = await chromium.launch();
  for (const [w, name] of [[1280, 'desk'], [400, 'phone']]) {
    const p = await b.newPage({ viewport: { width: w, height: 900 }, deviceScaleFactor: 1 });
    await p.goto('file://' + process.argv[2] + '/page/index.html');
    await p.waitForTimeout(800);
    const sw = await p.evaluate(() => document.documentElement.scrollWidth);
    console.log(name, 'scrollWidth', sw, 'height', await p.evaluate(() => document.body.scrollHeight));
    await p.screenshot({ path: process.argv[2] + `/look-${name}.png`, fullPage: false, clip: { x: 0, y: 0, width: w, height: name === 'desk' ? 3200 : 2600 } }).catch(async e => { await p.setViewportSize({ width: w, height: name === 'desk' ? 3200 : 2600 }); await p.screenshot({ path: process.argv[2] + `/look-${name}.png` }); });
  }
  await b.close();
})();
