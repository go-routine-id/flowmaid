// usage: node render.js <corpusDir> <outDir> <flowmaidBin>
const { chromium } = require('playwright');
const fs = require('fs'), path = require('path'), { execFileSync } = require('child_process');
const [corpus, out, bin] = process.argv.slice(2);
const mm = path.join(__dirname, '../mm/node_modules/mermaid/dist/mermaid.min.js');
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ deviceScaleFactor: 2, viewport: { width: 1600, height: 1000 } });
  await page.setContent('<html><body style="margin:0;background:#fff"><div id=c></div></body></html>');
  await page.addScriptTag({ path: mm });
  await page.evaluate(() => mermaid.initialize({ startOnLoad: false, securityLevel: 'loose' }));
  const files = fs.readdirSync(corpus).filter(f => f.endsWith('.mmd')).sort();
  const summary = [];
  for (const f of files) {
    const b = f.replace(/\.mmd$/, ''), src = fs.readFileSync(path.join(corpus, f), 'utf8');
    const rec = { name: b };
    // mermaid
    try {
      const svg = await page.evaluate(async (s) => { const { svg } = await mermaid.render('m' + Math.floor(Math.random()*1e9), s); return svg; }, src);
      fs.writeFileSync(path.join(out, b + '.mermaid.svg'), svg);
    } catch (e) { rec.mermaidErr = String(e).slice(0, 300); }
    // flowmaid
    try {
      const svg = execFileSync(bin, [path.join(corpus, f)], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
      fs.writeFileSync(path.join(out, b + '.flowmaid.svg'), svg);
    } catch (e) { rec.flowmaidErr = String(e.stderr || e).slice(0, 300); }
    summary.push(rec);
  }
  // screenshots + side-by-side
  const shot = await browser.newPage({ deviceScaleFactor: 1.5, viewport: { width: 1800, height: 800 } });
  for (const r of summary) {
    for (const k of ['mermaid', 'flowmaid']) {
      const p = path.join(out, `${r.name}.${k}.svg`);
      if (!fs.existsSync(p)) continue;
      await shot.setContent(`<html><body style="margin:0;background:#fff;display:inline-block"><div id=w style="display:inline-block;padding:8px">${fs.readFileSync(p,'utf8')}</div></body></html>`);
      await shot.evaluate(() => { const s = document.querySelector('#w svg'); const vb = s.viewBox && s.viewBox.baseVal; if (vb && vb.width) { s.setAttribute('width', vb.width); s.setAttribute('height', vb.height); s.style.maxWidth = 'none'; } });
      const box = await shot.$('#w svg');
      const bb = await box.boundingBox();
      r[k + 'Size'] = bb ? [Math.round(bb.width), Math.round(bb.height)] : null;
      await (await shot.$('#w')).screenshot({ path: path.join(out, `${r.name}.${k}.png`) });
    }
    const img = k => fs.existsSync(path.join(out, `${r.name}.${k}.png`)) ? `<img src="data:image/png;base64,${fs.readFileSync(path.join(out, r.name + '.' + k + '.png')).toString('base64')}" style="max-width:900px;max-height:1500px">` : `<div style="color:red">${(r[k+'Err']||'missing')}</div>`;
    await shot.setContent(`<html><body style="margin:0;background:#fff;font:14px sans-serif"><table id=t cellpadding=6 style="border-collapse:collapse"><tr><th>mermaid.js 12.2.0 — ${r.name}</th><th>flowmaid 0.32 — ${r.name}</th></tr><tr><td style="vertical-align:top;border-right:2px solid #ccc">${img('mermaid')}</td><td style="vertical-align:top">${img('flowmaid')}</td></tr></table></body></html>`);
    await shot.waitForLoadState('networkidle');
    await (await shot.$('#t')).screenshot({ path: path.join(out, `${r.name}.side.png`) });
  }
  fs.writeFileSync(path.join(out, 'summary.json'), JSON.stringify(summary, null, 1));
  console.log(JSON.stringify(summary));
  await browser.close();
})();
