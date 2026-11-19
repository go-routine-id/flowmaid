// node three.js <out> <labelA> <dirA> <labelB> <dirB> <labelC> <dirC> <names...>; dirs hold <name>.svg (mermaid dir holds <name>.mermaid.svg)
const { chromium } = require('playwright'); const fs = require('fs'), path = require('path');
(async () => {
  const [out, la, da, lb, db, lc, dc, ...names] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1.3, viewport: { width: 2400, height: 900 } });
  const svg = (d, n) => { for (const f of [n + '.svg', n + '.mermaid.svg', n + '.flowmaid.svg']) { const q = path.join(d, f); if (fs.existsSync(q)) return fs.readFileSync(q, 'utf8'); } return '<i>n/a</i>'; };
  for (const n of names) {
    await p.setContent(`<html><body style="margin:0;background:#fff;font:14px sans-serif"><table id=t cellpadding=6><tr><th>${la}</th><th>${lb}</th><th>${lc}</th></tr><tr>${[da, db, dc].map(d => `<td style="vertical-align:top;border-left:2px solid #ccc"><div class=w>${svg(d, n)}</div></td>`).join('')}</tr></table></body></html>`);
    await p.evaluate(() => { for (const s of document.querySelectorAll('.w > svg')) { const vb = s.viewBox && s.viewBox.baseVal; if (vb && vb.width) { s.setAttribute('width', vb.width); s.setAttribute('height', vb.height); s.style.maxWidth = 'none'; } } });
    await (await p.$('#t')).screenshot({ path: path.join(out, n + '.png') });
    console.log('ok', n);
  }
  await b.close();
})();
