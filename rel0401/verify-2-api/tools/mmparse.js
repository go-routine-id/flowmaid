// mermaid 12.2.0 parse check. usage: node mmparse.js <cases.json> -> one line per case: idx \t OK|ERR msg
const { chromium } = require('playwright');
const fs = require('fs');
const mm = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/mm/node_modules/mermaid/dist/mermaid.min.js';
(async () => {
  const cases = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
  const b = await chromium.launch(); const p = await b.newPage();
  await p.setContent('<html><body><div id=w></div></body></html>');
  await p.addScriptTag({ path: mm });
  await p.evaluate(() => mermaid.initialize({ startOnLoad: false }));
  console.error('mermaid version', await p.evaluate(() => (mermaid.version || (mermaid.default && mermaid.default.version) || '?')));
  const out = await p.evaluate(async cs => {
    const r = [];
    for (const s of cs) {
      try { await mermaid.parse(s); r.push('OK'); }
      catch (e) { r.push('ERR ' + String(e.message || e).split('\n').slice(0, 2).join(' | ').slice(0, 160)); }
    }
    return r;
  }, cases);
  out.forEach((x, i) => console.log(i + '\t' + x));
  await b.close();
})();
