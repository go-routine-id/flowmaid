// node three.js <outdir> <name>... : mermaid png | pre svg | now svg side by side -> out/<name>.three.png ; SCALE env
const { chromium } = require('playwright'); const fs = require('fs'), path = require('path');
(async () => {
  const [out, ...names] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: +(process.env.SCALE || 1.5), viewport: { width: 2400, height: 1000 } });
  for (const n of names) {
    const fix = (s, i) => fs.readFileSync(s, 'utf8').replace(/id="([^"]+)"/g, `id="c${i}_$1"`).replace(/url\(#([^)]+)\)/g, `url(#c${i}_$1)`);
    const mm = path.join(out, 'mm', n + '.mermaid.png');
    const im = fs.existsSync(mm) && !process.env.NOMM ? `<td style="vertical-align:top"><div>mermaid</div><img src="data:image/png;base64,${fs.readFileSync(mm).toString('base64')}" style="max-width:${process.env.MMW || 520}px"></td>` : '';
    await p.setContent(`<html><body style="margin:0;background:#fff;font:12px sans-serif"><table id=t cellpadding=6><tr>${im}<td style="vertical-align:top;border-left:2px solid #c00"><div>pre</div>${fix(path.join(out, 'pre', n + '.flowmaid.svg'), 1)}</td><td style="vertical-align:top;border-left:2px solid #c00"><div>now</div>${fix(path.join(out, 'now', n + '.flowmaid.svg'), 2)}</td></tr></table></body></html>`);
    await (await p.$('#t')).screenshot({ path: path.join(out, n + '.three.png') });
  }
  await b.close();
})();
