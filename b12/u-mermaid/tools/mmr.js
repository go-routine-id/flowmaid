// Render .mmd files with the bundled mermaid 12.2.0 under a named config.
// usage: node mmr.js <outDir> <tag> '<json config for mermaid.initialize>' <file.mmd>...
// writes <out>/<name>.<tag>.svg/.png and prints the effective layout.
const { chromium } = require('playwright');
const fs = require('fs'), path = require('path');
const mm = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/mm/node_modules/mermaid/dist/mermaid.min.js';
(async () => {
  const [out, tag, cfgJson, ...files] = process.argv.slice(2); fs.mkdirSync(out, { recursive: true });
  const cfg = Object.assign({ startOnLoad: false }, JSON.parse(cfgJson || '{}'));
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1.5, viewport: { width: 1600, height: 900 } });
  p.on('console', m => { if (process.env.MMLOG) console.log('[page]', m.text().slice(0, 300)); });
  await p.setContent('<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:8px"></div></body></html>');
  await p.addScriptTag({ path: mm }); await p.evaluate(c => mermaid.initialize(c), cfg);
  console.log('config.layout =', await p.evaluate(() => JSON.stringify({ layout: mermaid.mermaidAPI.getConfig().layout, look: mermaid.mermaidAPI.getConfig().look, theme: mermaid.mermaidAPI.getConfig().theme, fc: mermaid.mermaidAPI.getConfig().flowchart, elk: mermaid.mermaidAPI.getConfig().elk })));
  for (const f of files) {
    const n = path.basename(f).replace(/\.mmd$/, '');
    try {
      const svg = await p.evaluate(async s => (await mermaid.render('m' + Math.floor(Math.random() * 1e9), s)).svg, fs.readFileSync(f, 'utf8'));
      fs.writeFileSync(path.join(out, n + '.' + tag + '.svg'), svg);
      await p.evaluate(s => { const w = document.getElementById('w'); w.innerHTML = s; const e = w.querySelector('svg'); const vb = e.viewBox.baseVal; if (vb && vb.width) { e.setAttribute('width', vb.width); e.setAttribute('height', vb.height); e.style.maxWidth = 'none'; } }, svg);
      await (await p.$('#w')).screenshot({ path: path.join(out, n + '.' + tag + '.png') });
      console.log('ok', n, tag);
    } catch (e) { console.log('ERR', n, String(e).slice(0, 400)); }
  }
  await b.close();
})();
