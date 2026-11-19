// node png.js <out.png> <svg>...  : renders the SVGs side by side (with titles) into one PNG
const { chromium } = require("playwright"); const fs=require("fs"),path=require("path");
(async()=>{const [o,...f]=process.argv.slice(2);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:(+process.env.SCALE||1)});
const cells=f.map(x=>`<div style="display:inline-block;vertical-align:top;padding:6px;border-right:4px solid #888"><div style="font:12px sans-serif">${x.split('/').slice(-2).join('/')}</div>${fs.readFileSync(x,"utf8")}</div>`).join("");
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;white-space:nowrap">${cells}</div></body></html>`);
await (await p.$("#w")).screenshot({path:o});await b.close();})();
