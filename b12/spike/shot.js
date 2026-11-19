const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [,, ...f]=process.argv;const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1});
for(const x of f){const s=fs.readFileSync(x,"utf8");
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:8px">${s}</div></body></html>`);
await (await p.$("#w")).screenshot({path:x.replace(/\.svg$/,".png")});}await b.close();})();
