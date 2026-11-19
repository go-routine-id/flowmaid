// usage: node zoom.js <out.png> <scale> <x> <y> <w> <h> <name.svg> : crops scene region (svg user units) from pre and head, side by side
const { chromium } = require("playwright"); const fs=require("fs"),path=require("path");
(async()=>{const [o,scale,x,y,w,h,n]=process.argv.slice(2);const V=path.dirname(path.dirname(o));
const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:parseFloat(scale)});
const crop=(s)=>s.replace(/<svg([^>]*?)\swidth="[^"]*"/,'<svg$1').replace(/<svg([^>]*?)\sheight="[^"]*"/,'<svg$1').replace(/viewBox="[^"]*"/,`viewBox="${x} ${y} ${w} ${h}" width="${w}" height="${h}"`);
const a=crop(fs.readFileSync(path.join(V,"son-pre",n),"utf8")).replace(/flowmaid-/g,"pre-fm-"),hh=crop(fs.readFileSync(path.join(V,"son-head",n),"utf8")).replace(/flowmaid-/g,"head-fm-");
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-flex;gap:12px;padding:4px"><div style="border:1px solid #c00">${a}</div><div style="border:1px solid #0a0">${hh}</div></div></body></html>`);
await (await p.$("#w")).screenshot({path:o});await b.close();})();
