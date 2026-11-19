// node panel.js out.png label1=svg1 label2=svg2 ...
const { chromium } = require('playwright'); const fs = require('fs');
(async () => {
  const [out, ...items] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1.25, viewport: { width: 2400, height: 900 } });
  const cells = items.map(it => { const i = it.indexOf('='); const l = it.slice(0, i), f = it.slice(i + 1);
    const svg = fs.existsSync(f) ? fs.readFileSync(f, 'utf8') : '<i>missing</i>';
    return `<td style="vertical-align:top;border-left:2px solid #ccc"><div style="font:bold 14px sans-serif;padding:4px">${l}</div>${svg}</td>`; }).join('');
  await p.setContent(`<html><body style="margin:0;background:#fff"><table id=t cellpadding=4><tr>${cells}</tr></table></body></html>`);
  await p.evaluate(() => { for (const s of document.querySelectorAll('svg')) { const vb = s.viewBox && s.viewBox.baseVal; if (vb && vb.width) { const k = Math.min(1, 700 / vb.width); s.setAttribute('width', vb.width * k); s.setAttribute('height', vb.height * k); s.style.maxWidth = 'none'; } } });
  await (await p.$('#t')).screenshot({ path: out }); await b.close();
})();
