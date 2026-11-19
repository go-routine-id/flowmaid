// Rasterise pairs of SVGs in chromium and compare pixels.
// usage: node pixcmp.js <dirA> <dirB> <listfile>  -> one line per file: name diffPixels(>16) diffPixels(>64) maxDiff total
const { chromium } = require('playwright');
const fs = require('fs'), path = require('path');
(async () => {
  const [A, B, list] = process.argv.slice(2);
  const names = fs.readFileSync(list, 'utf8').split('\n').filter(Boolean);
  const b = await chromium.launch(); const p = await b.newPage();
  await p.setContent('<html><body></body></html>');
  for (const n of names) {
    const sa = fs.readFileSync(path.join(A, n), 'utf8'), sb = fs.readFileSync(path.join(B, n), 'utf8');
    const r = await p.evaluate(async ([sa, sb]) => {
      const load = s => new Promise((res, rej) => { const i = new Image(); i.onload = () => res(i); i.onerror = rej; i.src = URL.createObjectURL(new Blob([s], { type: 'image/svg+xml' })); });
      const wh = s => { const m = s.match(/<svg[^>]*width="([0-9.]+)" height="([0-9.]+)"/); return [Math.ceil(+m[1]), Math.ceil(+m[2])]; };
      const [w, h] = wh(sa); const [w2, h2] = wh(sb);
      if (w !== w2 || h !== h2) return 'SIZE ' + [w, h, w2, h2];
      const px = async s => { const c = document.createElement('canvas'); c.width = w; c.height = h; const x = c.getContext('2d'); x.drawImage(await load(s), 0, 0, w, h); return x.getImageData(0, 0, w, h).data; };
      const da = await px(sa), db = await px(sb);
      let d16 = 0, d64 = 0, mx = 0;
      for (let i = 0; i < da.length; i += 4) {
        const d = Math.max(Math.abs(da[i] - db[i]), Math.abs(da[i + 1] - db[i + 1]), Math.abs(da[i + 2] - db[i + 2]), Math.abs(da[i + 3] - db[i + 3]));
        if (d > 16) d16++; if (d > 64) d64++; if (d > mx) mx = d;
      }
      return [d16, d64, mx, w * h].join(' ');
    }, [sa, sb]);
    console.log(n + ' ' + r);
  }
  await b.close();
})();
