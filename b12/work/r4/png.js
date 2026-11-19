// node png.js <out.png> <label=svg>...   side-by-side picture of several SVGs
const { chromium } = require("playwright"); const fs = require("fs");
(async () => {
  const [out, ...items] = process.argv.slice(2);
  const cells = items.map(it => { const i = it.indexOf("="); const label = it.slice(0, i), f = it.slice(i + 1);
    return `<td style="vertical-align:top;padding:8px;border-right:2px solid #bbb"><div style="font:bold 16px sans-serif">${label}</div>${fs.readFileSync(f, "utf8")}</td>`; });
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: Number(process.env.SCALE || 1) });
  await p.setContent(`<html><body style="margin:0;background:#fff"><table id=w style="border-collapse:collapse"><tr>${cells.join("")}</tr></table></body></html>`);
  await (await p.$("#w")).screenshot({ path: out }); await b.close();
})();
