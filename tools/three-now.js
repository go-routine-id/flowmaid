const { chromium } = require('playwright'); const fs = require('fs'), path = require('path');
const [dir, ...names] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1, viewport: { width: 2400, height: 1000 } });
  const shot = async (svg, png) => { await p.setContent(`<html><body style="margin:0;background:#fff;display:inline-block"><div id=w style="display:inline-block;padding:8px">${fs.readFileSync(svg,'utf8')}</div></body></html>`);
    await p.evaluate(() => { const s = document.querySelector('#w svg'); const vb = s && s.viewBox && s.viewBox.baseVal; if (vb && vb.width) { s.setAttribute('width', vb.width); s.setAttribute('height', vb.height); s.style.maxWidth='none'; } });
    await (await p.$('#w')).screenshot({ path: png }); };
  for (const n of names) {
    for (const k of ['mermaid','default','blocks']) await shot(path.join(dir, `${n}.${k}.svg`), path.join(dir, `${n}.${k}.png`));
    const im = k => `<img src="data:image/png;base64,${fs.readFileSync(path.join(dir, `${n}.${k}.png`)).toString('base64')}" style="max-width:900px;max-height:1500px">`;
    await p.setContent(`<html><body style="margin:0;background:#fff;font:16px sans-serif"><table id=t cellpadding=6><tr><th>mermaid 12.2</th><th>flowmaid 0.40.2 (sekarang)</th><th>flowmaid mode blok (belum dirilis)</th></tr><tr><td style="vertical-align:top">${im('mermaid')}</td><td style="vertical-align:top;border-left:2px solid #ccc">${im('default')}</td><td style="vertical-align:top;border-left:2px solid #ccc">${im('blocks')}</td></tr></table></body></html>`);
    await (await p.$('#t')).screenshot({ path: path.join(dir, `${n}.three.png`) });
  }
  await b.close();
})();
