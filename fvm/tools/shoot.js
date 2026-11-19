// usage: node shoot.js <scale> <out_dir> <svg files...>  -> <out_dir>/<base>.png (scale x natural size)
const { chromium } = require('playwright'); const fs = require('fs'), path = require('path');
const [scale, outDir, ...files] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ deviceScaleFactor: Number(scale), viewport: { width: 4000, height: 3000 } });
  for (const f of files) {
    const svg = fs.readFileSync(f, 'utf8');
    await p.setContent(`<html><body style="margin:0;background:#fff"><div id="w" style="display:inline-block;background:#fff">${svg}</div></body></html>`);
    const box = await p.evaluate(() => { const s = document.querySelector('#w svg'); const vb = s.viewBox && s.viewBox.baseVal;
      let w = s.width.baseVal.value || (vb && vb.width), h = s.height.baseVal.value || (vb && vb.height);
      if (!w || !h) { const r = s.getBoundingClientRect(); w = r.width; h = r.height; }
      s.setAttribute('width', w); s.setAttribute('height', h); return { w, h }; });
    await p.setViewportSize({ width: Math.ceil(box.w) + 2, height: Math.ceil(box.h) + 2 });
    const base = path.basename(f).replace(/\.svg$/, '');
    await (await p.$('#w')).screenshot({ path: path.join(outDir, base + '.png') });
    console.log(base, Math.round(box.w), Math.round(box.h));
  }
  await b.close();
})();
