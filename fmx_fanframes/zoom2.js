const { chromium } = require('playwright');
const fs=require('fs');
const [svgPath, outPng, x, y, w, h, scale] = process.argv.slice(2);
(async()=>{
 const b=await chromium.launch(); const p=await b.newPage({deviceScaleFactor:Number(scale||4), viewport:{width:1200,height:2400}});
 let svg=fs.readFileSync(svgPath,'utf8');
 await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block">${svg}</div></body></html>`);
 await p.evaluate(()=>{const s=document.querySelector('#w svg');const vb=s.viewBox.baseVal;s.setAttribute('width',vb.width);s.setAttribute('height',vb.height);s.style.maxWidth='none';});
 const bb=await (await p.$('#w svg')).boundingBox();
 await p.screenshot({path:outPng, clip:{x:bb.x+Number(x),y:bb.y+Number(y),width:Number(w),height:Number(h)}});
 await b.close();
})();
