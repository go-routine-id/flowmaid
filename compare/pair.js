// mermaid | flowmaid side-by-side PNGs for the README.
// usage: FM_VERSION=0.35.0 node pair.js <corpusDir> <flowmaidBin> <mmDir> <fmDir> <outDir> name...
// e.g. node pair.js $S/cmp/corpus <tree>/target/release/flowmaid $S/cmp/out <scratch fm dir, must exist> <tree>/docs/img/compare advanced back-edge class demo er sequence state subgraph
const { chromium } = require('playwright');
const fs = require('fs'), path = require('path'), { execFileSync } = require('child_process');
const [corpus, bin, mmdir, fmdir, out, ...names] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1, viewport: { width: 1400, height: 900 } });
  const shot = async (svg, png) => {
    await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:10px">${svg}</div></body></html>`);
    await p.evaluate(() => { const s = document.querySelector('#w svg'); const vb = s.viewBox && s.viewBox.baseVal; if (vb && vb.width) { s.setAttribute('width', vb.width); s.setAttribute('height', vb.height); s.style.maxWidth = 'none'; } });
    await (await p.$('#w')).screenshot({ path: png });
  };
  for (const n of names) {
    const src = path.join(corpus, n + '.mmd');
    const fsvg = execFileSync(bin, [src], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
    fs.writeFileSync(path.join(fmdir, n + '.svg'), fsvg); await shot(fsvg, path.join(fmdir, n + '.png'));
    const img = f => 'data:image/png;base64,' + fs.readFileSync(f).toString('base64');
    await p.setContent(`<html><body style="margin:0;background:#fff;font:600 15px Inter,Helvetica,sans-serif;color:#101828">
      <table id=t style="border-collapse:collapse"><tr>
        <th style="padding:10px 14px 4px;text-align:left">mermaid 12.2.0</th><th style="padding:10px 14px 4px;text-align:left;border-left:1px solid #e3e7ee">flowmaid ${process.env.FM_VERSION || "?"}</th></tr>
      <tr><td style="padding:6px 14px 14px;vertical-align:top"><img src="${img(path.join(mmdir, n + '.mermaid.png'))}" style="max-width:620px;max-height:760px"></td>
          <td style="padding:6px 14px 14px;vertical-align:top;border-left:1px solid #e3e7ee"><img src="${img(path.join(fmdir, n + '.png'))}" style="max-width:620px;max-height:760px"></td></tr></table></body></html>`);
    await (await p.$('#t')).screenshot({ path: path.join(out, n + '.png') });
    console.log('ok', n);
  }
  await b.close();
})();
