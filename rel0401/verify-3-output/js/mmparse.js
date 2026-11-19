// node mmparse.js <cases.json> -> TSV: idx, case, mermaid parse result, class names
const { chromium } = require('playwright');
const fs = require('fs');
const mm = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/mm/node_modules/mermaid/dist/mermaid.min.js';
(async () => {
  const cases = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
  const b = await chromium.launch(); const p = await b.newPage();
  await p.setContent('<html><body><div id=w></div></body></html>');
  await p.addScriptTag({ path: mm });
  const v = await p.evaluate(() => { mermaid.initialize({ startOnLoad: false }); return mermaid.version || ''; });
  console.error('mermaid', v);
  for (let i = 0; i < cases.length; i++) {
    const src = 'classDiagram\n' + cases[i] + '\n';
    const r = await p.evaluate(async s => {
      try {
        const out = await mermaid.render('m' + Math.floor(Math.random() * 1e9), s);
        // class ids from the rendered svg
        const ids = [...out.svg.matchAll(/id="[^"]*classId-([^"]*?)-\d+"/g)].map(m => m[1]);
        return 'ok\t' + ids.join(',');
      } catch (e) { return 'ERR\t' + String(e.message || e).split('\n')[0].slice(0, 90); }
    }, src);
    console.log([i, JSON.stringify(cases[i]), r].join('\t'));
  }
  await b.close();
})();
