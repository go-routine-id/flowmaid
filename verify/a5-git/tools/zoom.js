// usage: node zoom.js <scale> <out.png> <in.svg>
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [sc,out,inp]=process.argv.slice(2);const b=await chromium.launch();
const p=await b.newPage({deviceScaleFactor:+sc});
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:4px;background:#fff">${fs.readFileSync(inp,"utf8")}</div></body></html>`);
await (await p.$("#w")).screenshot({path:out});await b.close();})();
