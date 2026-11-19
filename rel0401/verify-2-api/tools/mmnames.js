const { chromium } = require('playwright');
const fs = require('fs');
const mm = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/mm/node_modules/mermaid/dist/mermaid.min.js';
(async () => {
  const cases = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
  const b = await chromium.launch(); const p = await b.newPage();
  await p.setContent('<html><body><div id=w></div></body></html>');
  await p.addScriptTag({ path: mm });
  await p.evaluate(() => mermaid.initialize({ startOnLoad: false }));
  const out = await p.evaluate(async cs => {
    const r = [];
    for (const s of cs) {
      try {
        const d = await mermaid.mermaidAPI.getDiagramFromText(s);
        const cl = d.db.getClasses();
        const ids = [...cl.values()].map(c => c.id + (c.type ? '~' + c.type + '~' : '') + (c.label && c.label !== c.id ? '[' + c.label + ']' : ''));
        r.push('OK ' + JSON.stringify(ids));
      } catch (e) { r.push('RENDER-ERR ' + String(e.message || e).slice(0, 120)); }
    }
    return r;
  }, cases);
  out.forEach((x, i) => console.log(i + '\t' + x));
  await b.close();
})();
