// node crop.js out.png scale x y w h a.svg [b.svg ...]  -> side-by-side crops of the same region (SVG user units)
const { chromium } = require('playwright'); const fs = require('fs');
(async () => {
  const [out, scale, x, y, w, h, ...svgs] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: +scale, viewport: { width: 1900, height: 1000 } });
  const cells = svgs.map((s, i) => {
    let t = fs.readFileSync(s, 'utf8');
    // unique ids per copy (masks / markers)
    t = t.replace(/id="([^"]+)"/g, `id="c${i}_$1"`).replace(/url\(#([^)]+)\)/g, `url(#c${i}_$1)`).replace(/href="#([^"]+)"/g, `href="#c${i}_$1"`);
    t = t.replace(/<svg([^>]*)>/, (m, a) => { a = a.replace(/\swidth="[^"]*"/, '').replace(/\sheight="[^"]*"/, '').replace(/viewBox="[^"]*"/, ''); return `<svg${a} viewBox="${x} ${y} ${w} ${h}" width="${w}" height="${h}">`; });
    return `<td style="vertical-align:top;border:1px solid #c00;padding:0"><div style="font:10px sans-serif">${s.split('/').slice(-2).join('/')}</div>${t}</td>`;
  });
  await p.setContent(`<html><body style="margin:0;background:#fff"><table id=t cellspacing=4><tr>${cells.join('')}</tr></table></body></html>`);
  await (await p.$('#t')).screenshot({ path: out }); await b.close();
})();
