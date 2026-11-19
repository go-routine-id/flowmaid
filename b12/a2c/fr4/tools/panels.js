// node panels.js <out.png> <title1>=<file1.svg|png> <title2>=<file2> ...   (side by side; optional CROP=x0,y0,x1,y1 in svg units applied to svgs)
const { chromium } = require('playwright');
const fs = require('fs');
(async () => {
  const [out, ...items] = process.argv.slice(2);
  const browser = await chromium.launch();
  const page = await browser.newPage({ deviceScaleFactor: 1.5, viewport: { width: 1800, height: 800 } });
  const crop = process.env.CROP ? process.env.CROP.split(',').map(Number) : null;
  const scale = Number(process.env.SCALE || 1);
  const cells = items.map(it => {
    const i = it.indexOf('=');
    const title = it.slice(0, i), file = it.slice(i + 1);
    let body;
    if (file.endsWith('.png')) {
      body = `<img src="data:image/png;base64,${fs.readFileSync(file).toString('base64')}" style="max-width:900px">`;
    } else {
      let svg = fs.readFileSync(file, 'utf8');
      if (crop) {
        svg = svg.replace(/viewBox="[^"]*"/, `viewBox="${crop[0]} ${crop[1]} ${crop[2] - crop[0]} ${crop[3] - crop[1]}"`)
          .replace(/<svg([^>]*?) width="[^"]*"/, `<svg$1 width="${(crop[2] - crop[0]) * scale}"`)
          .replace(/<svg([^>]*?) height="[^"]*"/, `<svg$1 height="${(crop[3] - crop[1]) * scale}"`);
      }
      body = svg;
    }
    return `<td style="vertical-align:top;border-left:2px solid #ccc"><div style="font:14px sans-serif">${title}</div>${body}</td>`;
  });
  await page.setContent(`<html><body style="margin:0;background:#fff"><table id=t cellpadding=6><tr>${cells.join('')}</tr></table></body></html>`);
  if (!crop) {
    await page.evaluate(() => { for (const s of document.querySelectorAll('svg')) { const vb = s.viewBox && s.viewBox.baseVal; if (vb && vb.width) { s.setAttribute('width', vb.width); s.setAttribute('height', vb.height); s.style.maxWidth = 'none'; } } });
  }
  await (await page.$('#t')).screenshot({ path: out });
  await browser.close();
})();
