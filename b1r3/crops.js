// node crops.js out.png scale spec.json ; spec = [{svg, x, y, w, h, label}] (x,y = centre in svg coords)
const { chromium } = require("playwright"); const fs = require("fs");
(async () => {
  const [out, scale, specf] = process.argv.slice(2);
  const spec = JSON.parse(fs.readFileSync(specf, "utf8"));
  const br = await chromium.launch(); const p = await br.newPage({ deviceScaleFactor: parseFloat(scale) });
  let html = '<html><body style="margin:0;background:#fff"><div id=w style="display:flex;flex-wrap:wrap;gap:8px;padding:6px;width:1400px">';
  for (const c of spec) {
    let s = fs.readFileSync(c.svg, "utf8");
    const w = c.w || 60, h = c.h || 60;
    s = s.replace(/<svg([^>]*?)width="[^"]*"([^>]*?)height="[^"]*"([^>]*?)viewBox="[^"]*"/, `<svg$1width="${w*4}"$2height="${h*4}"$3viewBox="${c.x - w/2} ${c.y - h/2} ${w} ${h}"`);
    html += `<div><div style="font:11px sans-serif">${c.label}</div><img style="border:1px solid #ccc" src="data:image/svg+xml;base64,${Buffer.from(s).toString("base64")}"></div>`;
  }
  html += "</div></body></html>";
  await p.setContent(html); await p.waitForTimeout(150);
  await (await p.$("#w")).screenshot({ path: out }); await br.close();
})();
