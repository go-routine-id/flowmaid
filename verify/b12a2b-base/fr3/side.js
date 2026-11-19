// side.js <out.png> <scale> <svg>...: full SVGs side by side with labels
const { chromium } = require("playwright"); const fs = require("fs");
(async () => {
  const [o, sc, ...fs_] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: +sc });
  const parts = fs_.map(f => `<div style="background:#fff;padding:4px"><div style="font:12px sans-serif">${f.split("/").slice(-2).join("/")}</div>${fs.readFileSync(f, "utf8")}</div>`);
  await p.setContent(`<html><body style="margin:0;background:#888"><div id=w style="display:inline-flex;gap:6px;align-items:flex-start">${parts.join("")}</div></body></html>`);
  await (await p.$("#w")).screenshot({ path: o }); await b.close();
})();
