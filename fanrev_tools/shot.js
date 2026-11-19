const { chromium } = require('playwright');
const fs=require('fs');
const [svgPath, outPng, scale] = process.argv.slice(2);
(async()=>{
 const b=await chromium.launch(); const p=await b.newPage({deviceScaleFactor:Number(scale||1.5), viewport:{width:1400,height:1200}});
 let svg=fs.readFileSync(svgPath,'utf8');
 await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block">${svg}</div></body></html>`);
 const el=await p.$('#w svg'); await el.screenshot({path:outPng});
 await b.close();
})();
