// Independent "ink" oracle: does the browser see pickable paint where Rust
// picks something, and none where Rust picks nothing? Texts are taken out
// of the browser's hit test (their boxes are layout estimates); points near
// a text are not judged for "Rust hit, no paint".
const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');
const dir = process.argv[2];
const files = fs.readdirSync(dir).filter(f => f.endsWith('.json')).sort();
(async () => {
  let browser = await chromium.launch();
  let page = await browser.newPage();
  let k = 0;
  const tot = {};
  for (const f of files) {
   try {
    if (++k % 15 === 0) { try { await browser.close(); } catch (_) {} browser = await chromium.launch(); page = await browser.newPage(); }
    const J = JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8'));
    const S = Math.max(1, Math.min(2, 4000 / Math.max(J.w, J.h)));
    await page.setViewportSize({ width: Math.ceil(J.w * S) + 4, height: Math.ceil(J.h * S) + 4 });
    await page.setContent('<html><body style="margin:0;padding:0"><div id=w style="line-height:0"></div></body></html>');
    const res = await page.evaluate(([J, S]) => {
      const host = document.getElementById('w');
      host.innerHTML = J.svg;
      const svg = host.querySelector('svg');
      svg.setAttribute('width', String(J.w * S));
      svg.setAttribute('height', String(J.h * S));
      svg.style.display = 'block';
      // solid strokes (Rust treats dashed strokes as solid)
      for (const e of svg.querySelectorAll('[stroke-dasharray]')) e.removeAttribute('stroke-dasharray');
      // the background, titles and texts are not judged as paint
      const bg = svg.querySelector('rect:not([x])');
      if (bg) bg.style.pointerEvents = 'none';
      const texts = [...svg.querySelectorAll('text')];
      const tboxes = texts.map(t => { const b = t.getBBox(); return [b.x, b.y, b.width, b.height]; });
      for (const t of texts) t.style.pointerEvents = 'none';
      // non-pickable paint: the journey's path, an empty pie's outline ring
      if (J.type === 'journey') for (const e of svg.querySelectorAll('polyline')) e.style.pointerEvents = 'none';
      if (J.type === 'pie') for (const e of svg.querySelectorAll('circle[fill="none"]')) e.style.pointerEvents = 'none';
      const inked = (x, y) => {
        const el = document.elementFromPoint(x * S, y * S);
        return !!el && el !== svg && svg.contains(el);
      };
      const nearText = (x, y) => tboxes.some(([bx, by, bw, bh]) => {
        const padX = 6 + 0.25 * bw, padY = 6;
        return x >= bx - padX && x <= bx + bw + padX && y >= by - padY && y <= by + bh + padY;
      });
      const d = 0.3;
      const nb = [[d, 0], [-d, 0], [0, d], [0, -d]];
      const out = { file: J.file, type: J.type, n: 0, inkNoPick: 0, pickNoInk: 0, edge: 0, textSkip: 0, samples: [] };
      const N = J.pts.length / 2;
      for (let i = 0; i < N; i++) {
        const x = J.pts[2 * i], y = J.pts[2 * i + 1];
        const r = J.rust[i];
        const ink = inked(x, y);
        out.n++;
        if (ink === (r !== 'null')) continue;
        if (nb.some(([dx, dy]) => inked(x + dx, y + dy) !== ink)) { out.edge++; continue; }
        if (!ink && nearText(x, y)) { out.textSkip++; continue; }
        if (ink) out.inkNoPick++; else out.pickNoInk++;
        if (out.samples.length < 6) {
          const el = ink ? document.elementFromPoint(x * S, y * S) : null;
          out.samples.push(`(${x.toFixed(2)},${y.toFixed(2)}) rust=${r} ink=${ink}${el ? ' ' + el.outerHTML.slice(0, 120) : ''}`);
        }
      }
      return out;
    }, [J, S]);
    const t = tot[res.type] || (tot[res.type] = { files: 0, n: 0, inkNoPick: 0, pickNoInk: 0, edge: 0, textSkip: 0, badFiles: 0 });
    t.files++; t.n += res.n; t.inkNoPick += res.inkNoPick; t.pickNoInk += res.pickNoInk; t.edge += res.edge; t.textSkip += res.textSkip;
    if (res.inkNoPick || res.pickNoInk) { t.badFiles++; console.log(JSON.stringify(res)); }
   } catch (e) {
    console.log('ERROR ' + f + ' ' + String(e).slice(0, 160));
    try { await browser.close(); } catch (_) {}
    browser = await chromium.launch(); page = await browser.newPage();
   }
  }
  console.log('TOTALS ' + JSON.stringify(tot));
  await browser.close();
})();
