// usage: node zoom2.js <out.png> <scale> <x> <y> <w> <h> <a.svg> <b.svg> [c.svg]: crops a scene region (svg user units) from each svg, side by side
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [o,scale,x,y,w,h,...fs_]=process.argv.slice(2);
const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:parseFloat(scale)});
const crop=(s)=>s.replace(/<svg([^>]*?)\swidth="[^"]*"/,'<svg$1').replace(/<svg([^>]*?)\sheight="[^"]*"/,'<svg$1').replace(/viewBox="[^"]*"/,`viewBox="${x} ${y} ${w} ${h}" width="${w}" height="${h}"`);
const cols=["#c00","#0a0","#00c"];
const parts=fs_.map((f,i)=>`<div style="border:1px solid ${cols[i%3]}">${crop(fs.readFileSync(f,"utf8")).replace(/flowmaid-/g,"v"+i+"-fm-")}</div>`).join("");
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-flex;gap:12px;padding:4px">${parts}</div></body></html>`);
await (await p.$("#w")).screenshot({path:o});await b.close();})();
