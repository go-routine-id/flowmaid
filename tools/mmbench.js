// Time mermaid 12.2.0 render (parse + layout + SVG string) per .mmd file in headless Chromium.
// usage: NODE_PATH=$(npm root -g) node mmbench.js <runs> <file.mmd>...   -> TSV: name  median_ms  min_ms  svg_bytes
// Each file: 2 warm-up renders, then <runs> timed renders; reports median and min wall time of mermaid.render().
const { chromium } = require('playwright');
const fs = require('fs'), path = require('path');
const mm = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/mm/node_modules/mermaid/dist/mermaid.min.js';
(async () => {
  const [runsArg, ...files] = process.argv.slice(2); const runs = +runsArg || 10;
  const b = await chromium.launch(); const p = await b.newPage();
  await p.setContent('<html><body style="margin:0;background:#fff"></body></html>');
  await p.addScriptTag({ path: mm }); await p.evaluate(() => mermaid.initialize({ startOnLoad: false }));
  console.log('name\tmedian_ms\tmin_ms\tsvg_bytes');
  for (const f of files) {
    const n = path.basename(f).replace(/\.mmd$/, '');
    try {
      const r = await p.evaluate(async ([s, runs]) => {
        const ts = []; let bytes = 0;
        for (let i = 0; i < runs + 2; i++) {
          const t = performance.now();
          const { svg } = await mermaid.render('m' + i + '_' + Math.floor(Math.random() * 1e9), s);
          const dt = performance.now() - t;
          if (i >= 2) ts.push(dt);
          bytes = svg.length;
        }
        ts.sort((a, b) => a - b);
        return { med: ts[Math.floor(ts.length / 2)], min: ts[0], bytes };
      }, [fs.readFileSync(f, 'utf8'), runs]);
      console.log(`${n}\t${r.med.toFixed(2)}\t${r.min.toFixed(2)}\t${r.bytes}`);
    } catch (e) { console.log(`${n}\tERR\t${String(e).slice(0, 120)}\t0`); }
  }
  await b.close();
})();
