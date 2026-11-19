// usage: node multi.js <outDir> <mmdir> <file.mmd>...  ; env COLS="lab=bin[:FMX],..."
const { chromium } = require('playwright');
const fs = require('fs'), path = require('path'), { execFileSync } = require('child_process');
const [out, mmdir, ...files] = process.argv.slice(2);
const cols = process.env.COLS.split(',').map(c => { const [lab, rest] = c.split('='); const [bin, fmx] = rest.split(':'); return [lab, bin, fmx || '']; });
(async () => {
  fs.mkdirSync(out, { recursive: true });
  const b = await chromium.launch(); const page = await b.newPage({ deviceScaleFactor: 1, viewport: { width: 1800, height: 800 } });
  const shot = async (svg, png) => {
    await page.setContent(`<html><body style="margin:0;background:#fff;display:inline-block"><div id=w style="display:inline-block;padding:8px">${svg}</div></body></html>`);
    await page.evaluate(() => { const s = document.querySelector('#w svg'); const vb = s && s.viewBox && s.viewBox.baseVal; if (vb && vb.width) { s.setAttribute('width', vb.width); s.setAttribute('height', vb.height); s.style.maxWidth = 'none'; } });
    await (await page.$('#w')).screenshot({ path: png });
  };
  for (const f of files) {
    const n = path.basename(f).replace(/\.mmd$/, '');
    const cells = [];
    for (const cand of [path.join(mmdir, n + '.mermaid.png'), path.join(mmdir, n + '.png')]) if (fs.existsSync(cand)) { cells.push(['mermaid', cand]); break; }
    for (const [lab, bin, fmx] of cols) {
      try {
        const svg = execFileSync(bin, [f], { encoding: 'utf8', env: { ...process.env, FMX: fmx.replace(/\+/g, ',') }, stdio: ['ignore', 'pipe', 'pipe'], maxBuffer: 1 << 28 });
        fs.writeFileSync(path.join(out, `${n}.${lab}.svg`), svg);
        await shot(svg, path.join(out, `${n}.${lab}.png`));
        cells.push([lab, path.join(out, `${n}.${lab}.png`)]);
      } catch (e) { console.log('ERR', n, lab, String(e).slice(0, 200)); }
    }
    const W = Number(process.env.MAXW || 700);
    const im = p => fs.existsSync(p) ? `<img src="data:image/png;base64,${fs.readFileSync(p).toString('base64')}" style="max-width:${W}px;max-height:1800px">` : '<i>n/a</i>';
    await page.setContent(`<html><body style="margin:0;background:#fff;font:14px sans-serif"><table id=t cellpadding=4><tr>${cells.map(c => `<th>${c[0]}</th>`).join('')}</tr><tr>${cells.map(c => `<td style="vertical-align:top;border-left:2px solid #ccc">${im(c[1])}</td>`).join('')}</tr></table></body></html>`);
    await (await page.$('#t')).screenshot({ path: path.join(out, n + '.multi.png') });
    console.log('ok', n);
  }
  await b.close();
})();
