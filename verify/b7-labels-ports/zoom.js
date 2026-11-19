const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [svg,out,x,y,w,h,scale]=process.argv.slice(2);const b=await chromium.launch();
const p=await b.newPage({deviceScaleFactor:+(scale||3), viewport:{width:6000,height:6000}});
await p.setContent(`<html><body style="margin:0;background:#fff">${fs.readFileSync(svg,"utf8")}</body></html>`);
const el=await p.$("svg"); const bb=await el.boundingBox();
await p.screenshot({path:out,clip:{x:bb.x+(+x),y:bb.y+(+y),width:+w,height:+h}});await b.close();})();
