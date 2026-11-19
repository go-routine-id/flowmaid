// three.js <out.png> <svg>...: side by side, labelled with the parent dir name
const { chromium } = require("playwright"); const fs = require("fs"); const path = require("path");
(async () => {
  const [out, ...svgs] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1.5 });
  const cells = svgs.map(s => `<div style="display:inline-block;vertical-align:top;margin:4px;border-right:3px solid #888;padding:4px"><div style="font:12px sans-serif">${s.split('/').slice(-4).join('/')}</div>${fs.existsSync(s) ? fs.readFileSync(s, 'utf8') : 'missing'}</div>`).join('');
  await p.setContent(`<html><body style="margin:0;background:#fff;white-space:nowrap"><div id=w style="display:inline-block">${cells}</div></body></html>`);
  await (await p.$('#w')).screenshot({ path: out });
  await b.close();
})();
