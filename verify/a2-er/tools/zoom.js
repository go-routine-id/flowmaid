// usage: node zoom.js <svg> <out.png> <x> <y> <w> <h> [scale]
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [svg,out,x,y,w,h,sc]=process.argv.slice(2);const s=+(sc||5);
const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:s,viewport:{width:3000,height:3000}});
await p.setContent(`<html><body style="margin:0;background:#fff">${fs.readFileSync(svg,"utf8")}</body></html>`);
await p.screenshot({path:out,clip:{x:+x,y:+y,width:+w,height:+h}});await b.close();})();
