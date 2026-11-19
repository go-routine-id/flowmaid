// node mmparse.js <cases.json> : mermaid 12.2 parse + render result per case
const { chromium } = require('playwright');
const fs = require('fs');
const mm = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/mm/node_modules/mermaid/dist/mermaid.min.js';
(async () => {
  const cases = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
  const b = await chromium.launch(); const p = await b.newPage();
  await p.setContent('<html><body><div id=w></div></body></html>');
  await p.addScriptTag({ path: mm });
  const ver = await p.evaluate(() => { mermaid.initialize({ startOnLoad: false }); return mermaid.version || (window.mermaid && mermaid.mermaidAPI && 'n/a'); });
  console.error('mermaid', ver);
  for (let i = 0; i < cases.length; i++) {
    const src = 'classDiagram\n' + cases[i] + '\n';
    const r = await p.evaluate(async s => {
      let parse, render;
      try { await mermaid.parse(s); parse = 'ok'; } catch (e) { parse = 'ERR ' + String(e.message || e).split('\n').slice(0, 3).join(' | ').slice(0, 120); }
      try { await mermaid.render('m' + Math.floor(Math.random() * 1e9), s); render = 'ok'; } catch (e) { render = 'ERR ' + String(e.message || e).split('\n')[0].slice(0, 80); }
      return [parse, render];
    }, src);
    console.log([i, JSON.stringify(cases[i]), r[0], r[1]].join('\t'));
  }
  await b.close();
})();
