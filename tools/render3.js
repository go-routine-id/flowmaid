const { chromium } = require('playwright');
const fs = require('fs'), path = require('path'), { execFileSync } = require('child_process');
const [corpus, out, bin, mmdir, basedir] = process.argv.slice(2);
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ deviceScaleFactor: 1.5, viewport: { width: 1800, height: 800 } });
  const only = process.env.ONLY ? process.env.ONLY.split(',') : null;
  const files = fs.readdirSync(corpus).filter(f => f.endsWith('.mmd')).sort().filter(f => !only || only.includes(f.replace(/\.mmd$/,'')));
  const shot = async (svgPath, pngPath) => {
    await page.setContent(`<html><body style="margin:0;background:#fff;display:inline-block"><div id=w style="display:inline-block;padding:8px">${fs.readFileSync(svgPath,'utf8')}</div></body></html>`);
    await page.evaluate(() => { const s = document.querySelector('#w svg'); const vb = s && s.viewBox && s.viewBox.baseVal; if (vb && vb.width) { s.setAttribute('width', vb.width); s.setAttribute('height', vb.height); s.style.maxWidth = 'none'; } });
    await (await page.$('#w')).screenshot({ path: pngPath });
  };
  for (const f of files) {
    const b = f.replace(/\.mmd$/, '');
    try {
      const svg = execFileSync(bin, [path.join(corpus, f)], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
      fs.writeFileSync(path.join(out, b + '.svg'), svg);
      await shot(path.join(out, b + '.svg'), path.join(out, b + '.png'));
    } catch (e) { fs.writeFileSync(path.join(out, b + '.err'), String(e.stderr || e)); console.log('ERR', b); continue; }
    const im = p => fs.existsSync(p) ? `<img src="data:image/png;base64,${fs.readFileSync(p).toString('base64')}" style="max-width:800px;max-height:1600px">` : '<i>n/a</i>';
    const label = path.basename(out);
    await page.setContent(`<html><body style="margin:0;background:#fff;font:14px sans-serif"><table id=t cellpadding=6><tr><th>mermaid 12.2.0</th><th>flowmaid baseline (5f318a0)</th><th>flowmaid ${label}</th></tr><tr><td style="vertical-align:top">${im(path.join(mmdir, b + '.mermaid.png'))}</td><td style="vertical-align:top;border-left:2px solid #ccc">${im(path.join(basedir, b + '.png'))}</td><td style="vertical-align:top;border-left:2px solid #ccc">${im(path.join(out, b + '.png'))}</td></tr></table></body></html>`);
    await (await page.$('#t')).screenshot({ path: path.join(out, b + '.three.png') });
  }
  await browser.close();
})();
