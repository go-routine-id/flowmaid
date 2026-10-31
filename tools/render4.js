const { chromium } = require('/opt/node-tools/node_modules/playwright');
const fs = require('fs'), path = require('path'), { execFileSync } = require('child_process');
const [corpus, cmpOut, out, bin, ...names] = process.argv.slice(2);
(async () => {
  const browser = await chromium.launch();
  const shot = await browser.newPage({ deviceScaleFactor: 1.5, viewport: { width: 1800, height: 800 } });
  const snap = async (svg, p) => { await shot.setContent(`<html><body style="margin:0;background:#fff;display:inline-block"><div id=w style="display:inline-block;padding:8px">${svg}</div></body></html>`); await (await shot.$('#w')).screenshot({ path: p }); };
  for (const n of names) {
    const src = path.join(corpus, n + '.mmd');
    const a = execFileSync(bin, [src], { encoding: 'utf8' });
    const b = execFileSync(bin, [src], { encoding: 'utf8', env: { ...process.env, FM_ORTHO: '1' } });
    fs.writeFileSync(path.join(out, n + '.proto.svg'), a); fs.writeFileSync(path.join(out, n + '.ortho.svg'), b);
    await snap(a, path.join(out, n + '.proto.png')); await snap(b, path.join(out, n + '.ortho.png'));
    const img = p => `<img src="data:image/png;base64,${fs.readFileSync(p).toString('base64')}" style="max-width:560px;max-height:1300px">`;
    await shot.setContent(`<html><body style="margin:0;background:#fff;font:14px sans-serif"><table id=t cellpadding=6><tr><th>mermaid 12.2.0</th><th>flowmaid now</th><th>A: proposed tokens</th><th>B: tokens + rounded-orthogonal edges</th></tr><tr>${[path.join(cmpOut, n + '.mermaid.png'), path.join(cmpOut, n + '.flowmaid.png'), path.join(out, n + '.proto.png'), path.join(out, n + '.ortho.png')].map(p => `<td style="vertical-align:top;border-left:1px solid #ddd">${img(p)}</td>`).join('')}</tr></table></body></html>`);
    await (await shot.$('#t')).screenshot({ path: path.join(out, n + '.four.png') });
  }
  await browser.close();
})();
