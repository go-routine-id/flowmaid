// usage: node png.js <scale> <svg...>  -> writes <svg>.png
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [sc,...f]=process.argv.slice(2);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:+sc});
for(const s of f){await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:8px">${fs.readFileSync(s,"utf8")}</div></body></html>`);
await (await p.$("#w")).screenshot({path:s.replace(/\.svg$/,".png")});}await b.close();})();
