// Browser z-order oracle: hit-test the painter's own SVG (shapes tagged
// with their pick identity) with elementsFromPoint, compare to Rust.
const { chromium } = require('playwright');
const fs = require('fs');
const files = process.argv.slice(2);
(async () => {
  let browser = await chromium.launch();
  let page = await browser.newPage();
  const tot = {};
  for (const f of files) {
   try {
    const J = JSON.parse(fs.readFileSync(f, 'utf8'));
    if (process.env.KEEP_OUTLINE) J.keepOutline = true;
    const S = Math.max(1, Math.min(4, 8000 / Math.max(J.w, J.h), Math.sqrt(2.5e7 / (J.w * J.h))));
    await page.setViewportSize({ width: Math.ceil(J.w * S) + 4, height: Math.ceil(J.h * S) + 4 });
    await page.setContent('<html><body style="margin:0;padding:0"><div id=w style="line-height:0"></div></body></html>');
    const res = await page.evaluate(([J, S]) => {
      const host = document.getElementById('w');
      host.innerHTML = J.svg;
      const svg = host.querySelector('svg');
      svg.setAttribute('width', String(Math.ceil(J.w) * S));
      svg.setAttribute('height', String(Math.ceil(J.h) * S));
      svg.setAttribute('viewBox', `0 0 ${Math.ceil(J.w)} ${Math.ceil(J.h)}`);
      svg.style.display = 'block';
      // Rust's geometry: strokes solid (no dash gaps), filled shapes without their outline.
      for (const e of svg.querySelectorAll('[stroke-dasharray]')) e.removeAttribute('stroke-dasharray');
      if (!J.keepOutline) for (const e of svg.querySelectorAll('rect,circle,path,polygon,ellipse')) {
        const fl = e.getAttribute('fill');
        if (fl !== null && fl !== 'none') e.setAttribute('stroke', 'none');
      }
      const at = (x, y) => {
        const els = document.elementsFromPoint(x * S, y * S);
        for (const e of els) {
          const id = e.getAttribute && e.getAttribute('data-id');
          if (id) return id;
        }
        return 'null';
      };
      const near = [[0.2, 0], [-0.2, 0], [0, 0.2], [0, -0.2], [0.15, 0.15], [-0.15, 0.15], [0.15, -0.15], [-0.15, -0.15]];
      const pts = J.pts; const N = pts.length / 2;
      const out = { file: J.file, type: J.type, tagged: J.tagged, untagged: J.untagged, ambTags: J.amb, checked: 0, bad: 0, edge: 0, ambHit: 0, samples: [], kinds: {} };
      for (let i = 0; i < N; i++) {
        const x = pts[2 * i], y = pts[2 * i + 1];
        const b = at(x, y);
        const r = J.rust[i];
        out.checked++;
        if (b === r) continue;
        if (b.startsWith('AMB:') && b.slice(4).split('|').includes(r)) { out.ambHit++; continue; }
        if (near.some(([dx, dy]) => at(x + dx, y + dy) !== b)) { out.edge++; continue; }
        out.bad++;
        const k = `rust=${r.replace(/\d+/g, '#')} svg=${b.replace(/\d+/g, '#')}`;
        out.kinds[k] = (out.kinds[k] || 0) + 1;
        if (out.samples.length < 8) out.samples.push(`(${x.toFixed(2)},${y.toFixed(2)}) rust=${r} svg=${b}`);
      }
      return out;
    }, [J, S]);
    const t = tot[res.type] || (tot[res.type] = { files: 0, checked: 0, bad: 0, edge: 0, ambHit: 0, badFiles: 0 });
    t.files++; t.checked += res.checked; t.bad += res.bad; t.edge += res.edge; t.ambHit += res.ambHit; if (res.bad) t.badFiles++;
    if (res.bad || res.ambTags) console.log(JSON.stringify(res));
   } catch (e) {
    console.log('ERROR ' + f + ' ' + String(e).slice(0, 200));
    try { await browser.close(); } catch (_) {}
    browser = await chromium.launch();
    page = await browser.newPage();
   }
  }
  console.log('TOTALS ' + JSON.stringify(tot));
  await browser.close();
})();
