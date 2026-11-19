// three.js <out.png> <title1> <img-or-svg1> <title2> <img-or-svg2> [<title3> <img-or-svg3>]
const { chromium } = require('playwright'); const fs = require('fs');
(async () => {
  const [out, ...rest] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1.25, viewport: { width: 2400, height: 900 } });
  const cells = [];
  for (let i = 0; i < rest.length; i += 2) {
    const t = rest[i], f = rest[i + 1];
    let html = '<i>n/a</i>';
    if (fs.existsSync(f)) {
      if (f.endsWith('.svg')) html = fs.readFileSync(f, 'utf8');
      else html = `<img src="data:image/png;base64,${fs.readFileSync(f).toString('base64')}" style="max-width:900px">`;
    }
    cells.push(`<td style="vertical-align:top;border-left:2px solid #ccc"><div style="font:13px sans-serif">${t}</div>${html}</td>`);
  }
  await p.setContent(`<html><body style="margin:0;background:#fff"><table id=t cellpadding=6><tr>${cells.join('')}</tr></table></body></html>`);
  await p.evaluate(() => { for (const s of document.querySelectorAll('svg')) { const vb = s.viewBox && s.viewBox.baseVal; if (vb && vb.width) { s.setAttribute('width', vb.width); s.setAttribute('height', vb.height); s.style.maxWidth = 'none'; } } });
  await (await p.$('#t')).screenshot({ path: out });
  await b.close();
})();
