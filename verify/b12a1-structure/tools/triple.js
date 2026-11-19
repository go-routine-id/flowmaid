// usage: node triple.js <out.png> <label1>=<svg1> <label2>=<svg2> ...   (side by side, each scaled to fit)
const { chromium } = require('playwright'); const fs = require('fs');
(async () => {
  const [out, ...items] = process.argv.slice(2);
  const maxw = parseInt(process.env.MAXW || '900'), maxh = parseInt(process.env.MAXH || '1500');
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1 });
  const cells = items.map(it => { const i = it.indexOf('='); const label = it.slice(0, i), f = it.slice(i + 1);
    if (!fs.existsSync(f)) return `<td><b>${label}</b><br><i>n/a</i></td>`;
    let svg = fs.readFileSync(f, 'utf8');
    return `<td style="vertical-align:top;border-left:2px solid #ccc"><b>${label}</b><div class=c>${svg}</div></td>`; }).join('');
  await p.setContent(`<html><body style="margin:0;background:#fff;font:14px sans-serif"><table id=t cellpadding=6><tr>${cells}</tr></table></body></html>`);
  await p.evaluate(([maxw, maxh]) => { for (const s of document.querySelectorAll('.c > svg')) { const vb = s.viewBox.baseVal; let w = vb && vb.width ? vb.width : s.getBoundingClientRect().width, h = vb && vb.height ? vb.height : s.getBoundingClientRect().height; const k = Math.min(1, maxw / w, maxh / h); s.setAttribute('width', w * k); s.setAttribute('height', h * k); s.style.maxWidth = 'none'; } }, [maxw, maxh]);
  await (await p.$('#t')).screenshot({ path: out });
  await b.close();
})();
