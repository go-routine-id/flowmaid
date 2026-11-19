// mermaid parse check: node parse.js <cases.json> -> one line per case: OK [ids] | ERR msg
const { chromium } = require('playwright');
const fs = require('fs');
const mm = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/mm/node_modules/mermaid/dist/mermaid.min.js';
(async () => {
  const cases = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
  const b = await chromium.launch(); const p = await b.newPage();
  await p.setContent('<html><body><div id=w></div></body></html>');
  await p.addScriptTag({ path: mm });
  const ver = await p.evaluate(() => { mermaid.initialize({ startOnLoad: false }); return mermaid.version || (mermaid.mermaidAPI && mermaid.mermaidAPI.getConfig && 'n/a'); });
  console.error('mermaid', ver);
  for (const src of cases) {
    const r = await p.evaluate(async s => {
      try { await mermaid.parse(s); } catch (e) { return 'ERR ' + String(e.message || e).split('\n').slice(0, 3).join(' | ').slice(0, 160); }
      try { const { svg } = await mermaid.render('m' + Math.floor(Math.random() * 1e9), s);
        const ids = [...svg.matchAll(/id="[^"]*classId-([^"]*?)-\d+"/g)].map(m => m[1]);
        return 'OK ' + JSON.stringify(ids); } catch (e) { return 'RENDER-ERR ' + String(e.message || e).slice(0, 120); }
    }, src);
    console.log(JSON.stringify(src) + '\t' + r);
  }
  await b.close();
})();
