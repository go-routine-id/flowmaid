// zoom.js out.png scale "label1=svg1@x,y,w,h" ["label2=svg2@x,y,w,h" ...] — crops side by side.
const { chromium } = require("playwright"); const fs = require("fs");
(async () => {
  const [out, scale, ...items] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1 });
  let html = '<html><body style="margin:0;background:#fff;display:flex;gap:12px;font:14px sans-serif">';
  for (const it of items) {
    const [label, rest] = it.split('='); const [file, box] = rest.split('@');
    const [x, y, w, h] = box.split(',').map(Number);
    let svg = fs.readFileSync(file, 'utf8');
    svg = svg.replace(/<svg([^>]*?) width="[^"]*" height="[^"]*"/, '<svg$1').replace(/viewBox="[^"]*"/, `viewBox="${x} ${y} ${w} ${h}" width="${w*scale}" height="${h*scale}"`);
    html += `<div><div>${label}</div><div style="border:1px solid #ccc">${svg}</div></div>`;
  }
  html += '</body></html>';
  await p.setContent(html);
  await p.screenshot({ path: out, fullPage: true });
  await b.close();
})();
