// usage: node cmp.js <outdir> <maxw> <label=bin>... -- <files.mmd...>
const { chromium } = require('playwright');
const fs = require('fs'), path = require('path'), { execFileSync } = require('child_process');
const args = process.argv.slice(2); const out = args[0]; const maxw = +args[1];
const sep = args.indexOf('--'); const bins = args.slice(2, sep).map(s => s.split('=')); const files = args.slice(sep + 1);
fs.mkdirSync(out, { recursive: true });
(async () => {
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1.2, viewport: { width: 1800, height: 800 } });
  const shot = async (svg, png) => { await p.setContent(`<html><body style="margin:0;background:#fff;display:inline-block"><div id=w style="display:inline-block;padding:8px">${svg}</div></body></html>`);
    await p.evaluate(() => { const s = document.querySelector('#w svg'); const vb = s && s.viewBox && s.viewBox.baseVal; if (vb && vb.width) { s.setAttribute('width', vb.width); s.setAttribute('height', vb.height); s.style.maxWidth = 'none'; } });
    await (await p.$('#w')).screenshot({ path: png }); };
  for (const f of files) {
    const n = path.basename(f).replace(/\.mmd$/, ''); const cells = [];
    for (const [lab, bin] of bins) {
      let svg; try { svg = execFileSync(bin, [f], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }); } catch (e) { cells.push([lab, null]); continue; }
      const png = path.join(out, `${n}.${lab}.png`); fs.writeFileSync(path.join(out, `${n}.${lab}.svg`), svg); await shot(svg, png); cells.push([lab, png]);
    }
    const im = q => q ? `<img src="data:image/png;base64,${fs.readFileSync(q).toString('base64')}" style="max-width:${maxw}px;max-height:1600px">` : '<i>err</i>';
    await p.setContent(`<html><body style="margin:0;background:#fff;font:14px sans-serif"><table id=t cellpadding=6><tr>${cells.map(c => `<th>${n} ${c[0]}</th>`).join('')}</tr><tr>${cells.map(c => `<td style="vertical-align:top;border-left:2px solid #ccc">${im(c[1])}</td>`).join('')}</tr></table></body></html>`);
    await (await p.$('#t')).screenshot({ path: path.join(out, `${n}.cmp.png`) });
  }
  await b.close();
})();
