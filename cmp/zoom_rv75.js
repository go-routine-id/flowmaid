const { chromium } = require('playwright');
const fs=require('fs');
const [svgPath, outPng, x, y, w, h, scale] = process.argv.slice(2);
(async()=>{
 let svg=fs.readFileSync(svgPath,'utf8');
 const vbm=svg.match(/viewBox="([^"]*)"/)[1].split(/\s+/).map(Number);
 const b=await chromium.launch(); const p=await b.newPage({deviceScaleFactor:Number(scale||3), viewport:{width:Math.ceil(vbm[2])+20,height:Math.ceil(vbm[3])+20}});
 await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block">${svg}</div></body></html>`);
 await p.evaluate(()=>{const s=document.querySelector('#w svg');const vb=s.viewBox.baseVal;s.setAttribute('width',vb.width);s.setAttribute('height',vb.height);s.style.maxWidth='none';});
 const bb=await (await p.$('#w svg')).boundingBox();
 await p.screenshot({path:outPng, clip:{x:bb.x+Number(x),y:bb.y+Number(y),width:Number(w),height:Number(h)}});
 await b.close();
})();
