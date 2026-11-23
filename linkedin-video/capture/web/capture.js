// Records the flowmaid-web playground: one node dragged across the canvas,
// one page.screenshot per 30 fps frame. Real page, real wasm engine; the only
// thing injected is a drawn cursor (headless Chromium paints no pointer).
const c = require('./common.js');
const fs = require('fs');
const path = require('path');
const OUT = process.argv[2];
const SRC = process.argv[3] || 'd1.mmd';
fs.mkdirSync(OUT, { recursive: true });
const CLIP = { x: 220, y: 56, width: 1060, height: 709.3334 }; // code panel + canvas
const ease = (t) => (t < .5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2);
const lerp = (a, b, t) => a + (b - a) * t;

(async () => {
  const { browser, page } = await c.open({ src: SRC, w: 1280, h: 800, zoom100: true, place: [null, 28, 40] });
  // Drawn cursor (pointer-events: none, so it never takes part in hit tests).
  await page.evaluate(() => {
    const d = document.createElement('div');
    d.id = '__cursor';
    d.style.cssText = 'position:fixed;left:0;top:0;z-index:2147483647;pointer-events:none;width:0;height:0;';
    d.innerHTML =
      '<div id="__ring" style="position:absolute;left:-14px;top:-14px;width:28px;height:28px;border-radius:50%;' +
      'background:rgba(136,57,239,.10);border:2px solid rgba(136,57,239,.6);box-sizing:border-box;display:none"></div>' +
      '<svg width="27" height="27" viewBox="0 0 24 24" style="position:absolute;left:-5.6px;top:-3.4px;' +
      'filter:drop-shadow(0 1.5px 2px rgba(0,0,0,.35))">' +
      '<path d="M5 3 L5 20.2 L9.6 15.9 L12.7 22.6 L15.4 21.4 L12.4 14.9 L18.6 14.9 Z" fill="#141414" stroke="#fff" ' +
      'stroke-width="1.6" stroke-linejoin="round"/></svg>';
    document.body.appendChild(d);
    window.__cur = (x, y, down) => {
      d.style.transform = `translate(${x}px, ${y}px)`;
      document.getElementById('__ring').style.display = down ? 'block' : 'none';
    };
  });
  const view = await page.evaluate(() => { const r = document.getElementById('view').getBoundingClientRect(); return [r.left, r.top, r.width, r.height]; });
  const auth = await c.nodeCenter(page, 'Auth?');
  const authBox = await page.evaluate(() => {
    // the diamond <polygon>/<path> under the "Auth?" label: take the widest shape whose box holds the label centre
    const svg = document.querySelector('#stage svg');
    const t = [...svg.querySelectorAll('text')].find((e) => e.textContent.trim() === 'Auth?').getBoundingClientRect();
    const cx = t.left + t.width / 2, cy = t.top + t.height / 2;
    let best = null;
    for (const el of svg.querySelectorAll('polygon, path, rect')) {
      const r = el.getBoundingClientRect();
      if (r.left < cx && r.right > cx && r.top < cy && r.bottom > cy && r.width < 200 && r.height < 120) {
        if (!best || r.width > best.width) best = r;
      }
    }
    return best && [best.left, best.top, best.width, best.height];
  });
  console.log('view', view, 'auth', auth, 'authBox', authBox);
  // Drag target: Auth? ends with its left edge ~34 px inside the canvas, a touch lower.
  const halfW = authBox ? authBox[2] / 2 : 40;
  // grab a little right of / below the label so the cursor does not hide it
  const grab = [auth[0] + 13, auth[1] + 9];
  const target = [view[0] + 34 + halfW + 13, auth[1] + 9 + 40];
  const rest = [view[0] + view[2] - 60, auth[1] + 70];
  const snap = (v) => Math.round(v * 3) / 3; // whole device pixels at DSF 1.5
  const plan = [];
  const T0 = 18, MOVE = 24, HOVER = 10, PRESS = 5, DRAG = 60, HOLD1 = 18, AWAY = 16, HOLD2 = 40;
  for (let i = 0; i < T0; i++) plan.push({ p: rest, down: false });
  for (let i = 1; i <= MOVE; i++) {
    const t = ease(i / MOVE);
    // slight curve on the approach, like a hand on a trackpad
    plan.push({ p: [lerp(rest[0], grab[0], t), lerp(rest[1], grab[1], t) - 18 * Math.sin(Math.PI * t)], down: false });
  }
  for (let i = 0; i < HOVER; i++) plan.push({ p: grab, down: false });
  for (let i = 0; i < PRESS; i++) plan.push({ p: grab, down: true, press: i === 0 });
  for (let i = 1; i <= DRAG; i++) {
    const t = ease(i / DRAG);
    plan.push({ p: [lerp(grab[0], target[0], t), lerp(grab[1], target[1], t) - 30 * Math.sin(Math.PI * t)], down: true });
  }
  // Release, keep the hover toolbar up for a moment, then drift the pointer off
  // into empty canvas so the result reads clean (selection stays).
  for (let i = 0; i < HOLD1; i++) plan.push({ p: target, down: false, release: i === 0 });
  const away = [view[0] + 64, view[1] + 470];
  for (let i = 1; i <= AWAY; i++) {
    const t = ease(i / AWAY);
    plan.push({ p: [lerp(target[0], away[0], t) + 14 * Math.sin(Math.PI * t), lerp(target[1], away[1], t)], down: false });
  }
  for (let i = 0; i < HOLD2; i++) plan.push({ p: away, down: false });

  let last = [snap(rest[0]), snap(rest[1])];
  await page.mouse.move(last[0], last[1]);
  await c.raf2(page);
  const log = [];
  let n = 0;
  for (const f of plan) {
    const x = snap(f.p[0]), y = snap(f.p[1]);
    // A still hand sends no pointer events; re-sending the same point would keep
    // re-arming the page's hover timers.
    if (x !== last[0] || y !== last[1]) await page.mouse.move(x, y);
    last = [x, y];
    if (f.press) await page.mouse.down();
    if (f.release) await page.mouse.up();
    await page.evaluate(([x, y, d]) => window.__cur(x, y, d), [x, y, f.down]);
    await c.raf2(page);
    n++;
    const name = path.join(OUT, `f${String(n).padStart(4, '0')}.png`);
    await page.screenshot({ path: name, clip: CLIP });
    // Edge geometry as drawn by the engine this frame (for verification).
    const edges = await page.evaluate(() => {
      const o = {};
      for (const p of document.querySelectorAll('#stage svg > path')) {
        const t = p.querySelector('title')?.textContent;
        if (t) o[t] = p.getAttribute('d');
      }
      return o;
    });
    log.push({ frame: n, mouse: [x, y], down: f.down, edges });
  }
  fs.writeFileSync(path.join(OUT, '..', 'web-edge-log.json'), JSON.stringify(log));
  console.log('frames', n, 'target', target);
  await browser.close();
})();
