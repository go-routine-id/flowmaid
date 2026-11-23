const { chromium } = require('playwright');
const fs = require('fs');
exports.open = async function (opts) {
  const browser = await chromium.launch();
  const ctx = await browser.newContext({
    viewport: { width: opts.w, height: opts.h }, deviceScaleFactor: opts.dsf || 1.5,
    colorScheme: opts.scheme || 'light', reducedMotion: 'reduce',
  });
  const page = await ctx.newPage();
  page.on('pageerror', e => console.log('pageerror', e.message));
  await page.goto('http://127.0.0.1:8731/', { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(() => document.getElementById('version').textContent.includes('engine'));
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(300);
  await page.click('#editor');
  await page.fill('#editor', fs.readFileSync(opts.src, 'utf8').trimEnd());
  await page.waitForTimeout(400);
  await page.click('#relayout');
  await page.click('#zoom-fit');
  if (opts.zoom100) await page.click('#zoom-pct');
  for (let i = 0; i < (opts.zoomOut || 0); i++) await page.click('#zoom-out');
  await page.waitForTimeout(100);
  if (opts.place) {
    // Pan (wheel over the canvas = cam -= delta) so the diagram's top-left lands
    // at view coords opts.place = [x, y], or centre horizontally when x is null.
    const [dx, dy] = await page.evaluate((place) => {
      const v = document.getElementById('view').getBoundingClientRect();
      const s = document.querySelector('#stage svg').getBoundingClientRect();
      const tx = place[0] == null ? v.left + (v.width - s.width) / 2 + (place[2] || 0) : v.left + place[0];
      const ty = v.top + place[1];
      return [s.left - tx, s.top - ty];
    }, opts.place);
    await page.mouse.move(opts.w - 60, 300);
    await page.mouse.wheel(dx, dy);
  }
  await page.mouse.move(opts.w - 5, opts.h - 5);
  await page.evaluate(() => { document.activeElement && document.activeElement.blur(); window.getSelection().removeAllRanges(); });
  await page.waitForTimeout(400);
  return { browser, page };
};
exports.nodeCenter = async function (page, label) {
  return await page.evaluate((label) => {
    const svg = document.querySelector('#stage svg');
    for (const t of svg.querySelectorAll('text')) {
      if (t.textContent.trim() === label) {
        const r = t.getBoundingClientRect();
        return [r.left + r.width / 2, r.top + r.height / 2];
      }
    }
    return null;
  }, label);
};
exports.raf2 = (page) => page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
