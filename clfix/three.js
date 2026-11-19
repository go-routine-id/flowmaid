// node three.js <out.png> <label:svg>...  -> side-by-side screenshot
const { chromium } = require("playwright"); const fs = require("fs");
(async () => {
  const [out, ...items] = process.argv.slice(2);
  const cells = items.map(it => { const i = it.indexOf(":"); return `<div style="display:inline-block;vertical-align:top;margin:8px;border-right:1px solid #ccc;padding-right:8px"><div style="font:bold 13px sans-serif;margin-bottom:6px">${it.slice(0, i)}</div>${fs.readFileSync(it.slice(i + 1), "utf8")}</div>`; }).join("");
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1.2, viewport: { width: 2400, height: 900 } });
  await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;white-space:nowrap">${cells}</div></body></html>`);
  await (await p.$("#w")).screenshot({ path: out }); await b.close();
})();
