const c = require('./common.js');
(async () => {
  const [src, label, path, tag, scheme] = process.argv.slice(2);
  const pts = JSON.parse(path); // list of [dx,dy] waypoints relative to the node centre
  const { browser, page } = await c.open({ src, w: 1280, h: 800, zoom100: true, place: [null, 28, 40], scheme });
  await page.screenshot({ path: `t-${tag}-0.png` });
  const p = await c.nodeCenter(page, label);
  await page.mouse.move(p[0], p[1], { steps: 5 });
  await page.mouse.down();
  const N = 48;
  for (let i = 1; i <= N; i++) {
    const t = i / N, e = t < .5 ? 2*t*t : 1 - Math.pow(-2*t+2, 2)/2;
    // piecewise-linear through waypoints
    const segs = pts.length, u = e * segs, k = Math.min(segs - 1, Math.floor(u)), f = u - k;
    const a = k === 0 ? [0, 0] : pts[k - 1], b = pts[k];
    await page.mouse.move(p[0] + a[0] + (b[0]-a[0])*f, p[1] + a[1] + (b[1]-a[1])*f);
    await c.raf2(page);
    if (i === 16 || i === 32) await page.screenshot({ path: `t-${tag}-${i/16}.png` });
  }
  await page.mouse.up();
  await c.raf2(page);
  await page.screenshot({ path: `t-${tag}-3.png` });
  await browser.close();
})();
