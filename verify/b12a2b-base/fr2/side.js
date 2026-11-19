// side.js <out.png> <scale> <label=file>...  (svg or png files side by side with captions)
const { chromium } = require("playwright"); const fs = require("fs");
(async () => {
  const [o, scale, ...items] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: parseFloat(scale) });
  let html = '<html><body style="margin:0;background:#fff;font:14px sans-serif"><div id=w style="display:inline-flex;gap:16px;align-items:flex-start;padding:8px">';
  for (const it of items) {
    const i = it.indexOf('='); const lab = it.slice(0, i), f = it.slice(i + 1);
    let body;
    if (f.endsWith('.svg')) body = fs.readFileSync(f, 'utf8');
    else body = `<img src="data:image/png;base64,${fs.readFileSync(f).toString('base64')}" style="max-width:900px">`;
    html += `<div style="border-right:1px solid #aaa;padding-right:12px"><div style="font-weight:bold">${lab}</div>${body}</div>`;
  }
  html += '</div></body></html>';
  await p.setContent(html); await p.waitForTimeout(100);
  await (await p.$('#w')).screenshot({ path: o }); await b.close();
})();
