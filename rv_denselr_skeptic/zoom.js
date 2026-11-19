const { chromium } = require('playwright');
const fs=require('fs');
const [svgPath, outPng, x, y, w, h, scale] = process.argv.slice(2);
(async()=>{
 const b=await chromium.launch(); const p=await b.newPage({deviceScaleFactor:Number(scale||4), viewport:{width:Number(w),height:Number(h)}});
 let svg=fs.readFileSync(svgPath,'utf8');
 svg=svg.replace(/viewBox="[^"]*"/, `viewBox="${x} ${y} ${w} ${h}"`).replace(/width="\d+(\.\d+)?" height="\d+(\.\d+)?"/, `width="${w}" height="${h}"`);
 await p.setContent(`<html><body style="margin:0;background:#fff">${svg}</body></html>`);
 await p.screenshot({path:outPng, clip:{x:0,y:0,width:Number(w),height:Number(h)}});
 await b.close();
})();
