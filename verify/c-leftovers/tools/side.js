// side.js <out.png> <label=file.svg|file.png>... : panels side by side on one page.
const { chromium } = require("playwright"); const fs = require("fs");
(async () => {
  const [out, ...items] = process.argv.slice(2);
  const scale = parseFloat(process.env.SCALE || "1");
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: scale });
  let html = '<html><body style="margin:0;background:#fff;font-family:sans-serif"><div id=w style="display:inline-flex;gap:24px;padding:8px;align-items:flex-start">';
  for (const it of items) {
    const i = it.indexOf("="); const label = it.slice(0, i), f = it.slice(i + 1);
    let body = "<i>missing</i>";
    if (fs.existsSync(f) && fs.statSync(f).size) {
      body = f.endsWith(".svg") ? fs.readFileSync(f, "utf8") : `<img src="data:image/png;base64,${fs.readFileSync(f).toString("base64")}" style="zoom:${process.env.IMGZOOM || 0.66}">`;
    }
    html += `<div style="border-right:1px solid #bbb;padding-right:24px"><div style="font-weight:bold;font-size:13px;margin-bottom:6px">${label}</div>${body}</div>`;
  }
  html += "</div></body></html>";
  await p.setContent(html);
  await (await p.$("#w")).screenshot({ path: out });
  await b.close();
})();
