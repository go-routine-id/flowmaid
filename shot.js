const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [out,scale,...svgs]=process.argv.slice(2);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:+scale});
const body=svgs.map(s=>`<div style="display:inline-block;margin:6px;border:1px solid #ccc;vertical-align:top"><div style="font:11px sans-serif">${s.split('/').slice(-2).join('/')}</div>${fs.readFileSync(s,"utf8")}</div>`).join("");
await p.setContent(`<html><body style="margin:0;background:#fff">${body}</body></html>`);
await p.screenshot({path:out,fullPage:true});await b.close();})();
