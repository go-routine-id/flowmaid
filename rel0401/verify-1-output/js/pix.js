// node pix.js <pairs.tsv> <outdir> [shots]
// pairs.tsv: name \t svgA \t svgB ; prints name \t W x H \t differing pixels (>16 per channel)
const { chromium } = require('playwright');
const fs = require('fs'), path = require('path');
const [pairs, out, shots] = process.argv.slice(2);
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  await page.setContent('<html><body></body></html>');
  for (const line of fs.readFileSync(pairs, 'utf8').split('\n').filter(Boolean)) {
    const [name, a, b] = line.split('\t');
    const A = fs.readFileSync(a, 'utf8'), B = fs.readFileSync(b, 'utf8');
    const r = await page.evaluate(async ([A, B]) => {
      const load = s => new Promise((res, rej) => { const i = new Image(); i.onload = () => res(i); i.onerror = rej; i.src = 'data:image/svg+xml;base64,' + btoa(unescape(encodeURIComponent(s))); });
      const [ia, ib] = await Promise.all([load(A), load(B)]);
      const w = Math.max(ia.width, ib.width), h = Math.max(ia.height, ib.height);
      const px = im => { const c = document.createElement('canvas'); c.width = w; c.height = h; const x = c.getContext('2d'); x.fillStyle = '#fff'; x.fillRect(0, 0, w, h); x.drawImage(im, 0, 0); return x.getImageData(0, 0, w, h).data; };
      const da = px(ia), db = px(ib);
      let n = 0, big = 0;
      for (let k = 0; k < da.length; k += 4) { const m = Math.max(Math.abs(da[k]-db[k]), Math.abs(da[k+1]-db[k+1]), Math.abs(da[k+2]-db[k+2])); if (m > 16) n++; if (m > 128) big++; }
      return [ia.width, ia.height, ib.width, ib.height, n, big];
    }, [A, B]);
    console.log(`${name}\t${r[0]}x${r[1]}\t${r[2]}x${r[3]}\t${r[4]}\t${r[5]}`);
    if (shots) {
      const b64 = s => Buffer.from(s).toString('base64');
      await page.setContent(`<html><body style="margin:0;background:#fff;font:13px sans-serif"><table id=t cellpadding=6><tr><th>origin/dev: ${name}</th><th>HEAD</th></tr><tr><td style="vertical-align:top"><img src="data:image/svg+xml;base64,${b64(A)}"></td><td style="vertical-align:top;border-left:2px solid #ccc"><img src="data:image/svg+xml;base64,${b64(B)}"></td></tr></table></body></html>`);
      await (await page.$('#t')).screenshot({ path: path.join(out, name + '.png') });
      await page.setContent('<html><body></body></html>');
    }
  }
  await browser.close();
})();
