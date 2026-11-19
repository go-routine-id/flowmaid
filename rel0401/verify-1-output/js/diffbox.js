// node diffbox.js a.svg b.svg out.png : finds bbox of strongly differing pixels (>128), writes crop of A|B at 3x around it
const { chromium } = require('playwright');
const fs = require('fs');
const [a, b, out] = process.argv.slice(2);
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  await page.setContent('<html><body></body></html>');
  const A = fs.readFileSync(a, 'utf8'), B = fs.readFileSync(b, 'utf8');
  const r = await page.evaluate(async ([A, B]) => {
    const load = s => new Promise((res, rej) => { const i = new Image(); i.onload = () => res(i); i.onerror = rej; i.src = 'data:image/svg+xml;base64,' + btoa(unescape(encodeURIComponent(s))); });
    const [ia, ib] = await Promise.all([load(A), load(B)]);
    const w = ia.width, h = ia.height;
    const px = im => { const c = document.createElement('canvas'); c.width = w; c.height = h; const x = c.getContext('2d'); x.fillStyle = '#fff'; x.fillRect(0, 0, w, h); x.drawImage(im, 0, 0); return x.getImageData(0, 0, w, h).data; };
    const da = px(ia), db = px(ib);
    const pts = [];
    for (let k = 0; k < da.length; k += 4) { const m = Math.max(Math.abs(da[k]-db[k]), Math.abs(da[k+1]-db[k+1]), Math.abs(da[k+2]-db[k+2])); if (m > 128) pts.push([(k/4) % w, Math.floor(k/4/w)]); }
    return { w, h, pts };
  }, [A, B]);
  console.log(JSON.stringify(r.pts.slice(0, 80)));
  if (!r.pts.length) { await browser.close(); return; }
  const xs = r.pts.map(p => p[0]), ys = r.pts.map(p => p[1]);
  const x0 = Math.max(0, Math.min(...xs) - 40), y0 = Math.max(0, Math.min(...ys) - 40), x1 = Math.min(r.w, Math.max(...xs) + 40), y1 = Math.min(r.h, Math.max(...ys) + 40);
  const b64 = s => Buffer.from(s).toString('base64');
  const crop = s => `<div style="width:${(x1-x0)*3}px;height:${(y1-y0)*3}px;overflow:hidden;position:relative;display:inline-block;border:1px solid #888;margin:4px"><img src="data:image/svg+xml;base64,${b64(s)}" style="position:absolute;left:${-x0*3}px;top:${-y0*3}px;width:${r.w*3}px;height:${r.h*3}px"></div>`;
  await page.setContent(`<html><body style="margin:0;background:#fff"><div id=t>${crop(A)}${crop(B)}</div></body></html>`);
  await (await page.$('#t')).screenshot({ path: out });
  await browser.close();
})();
