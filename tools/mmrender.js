// Render arbitrary .mmd files with mermaid 12.2.0 -> <out>/<name>.mermaid.svg/.png
// usage: node mmrender.js <outDir> <file.mmd>...
const { chromium } = require('playwright');
const fs = require('fs'), path = require('path');
const mm = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/mm/node_modules/mermaid/dist/mermaid.min.js';
(async () => {
  const [out, ...files] = process.argv.slice(2); fs.mkdirSync(out, { recursive: true });
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1.5, viewport: { width: 1600, height: 900 } });
  await p.setContent('<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:8px"></div></body></html>');
  await p.addScriptTag({ path: mm }); await p.evaluate(() => mermaid.initialize({ startOnLoad: false }));
  for (const f of files) {
    const n = path.basename(f).replace(/\.mmd$/, '');
    try {
      const svg = await p.evaluate(async s => (await mermaid.render('m' + Math.floor(Math.random() * 1e9), s)).svg, fs.readFileSync(f, 'utf8'));
      fs.writeFileSync(path.join(out, n + '.mermaid.svg'), svg);
      await p.evaluate(s => { const w = document.getElementById('w'); w.innerHTML = s; const e = w.querySelector('svg'); const vb = e.viewBox.baseVal; if (vb && vb.width) { e.setAttribute('width', vb.width); e.setAttribute('height', vb.height); e.style.maxWidth = 'none'; } }, svg);
      await (await p.$('#w')).screenshot({ path: path.join(out, n + '.mermaid.png') });
      console.log('ok', n);
    } catch (e) { console.log('ERR', n, String(e).slice(0, 200)); }
  }
  await b.close();
})();
