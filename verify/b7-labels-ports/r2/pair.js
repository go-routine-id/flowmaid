// usage: node pair.js out.png left.svg right.svg [titleL] [titleR] [scale]
const { chromium } = require('playwright'); const fs=require('fs');
(async()=>{const [out,l,r,tl,tr,sc]=process.argv.slice(2);const b=await chromium.launch();
const p=await b.newPage({deviceScaleFactor:+(sc||1.5),viewport:{width:2400,height:1000}});
const fix=s=>s;
await p.setContent(`<html><body style="margin:0;background:#fff;font:14px sans-serif"><table id=t cellpadding=6><tr><th>${tl||'pre'}</th><th>${tr||'now'}</th></tr><tr><td style="vertical-align:top">${fs.readFileSync(l,'utf8')}</td><td style="vertical-align:top;border-left:2px solid #ccc">${fs.readFileSync(r,'utf8')}</td></tr></table></body></html>`);
await p.evaluate(()=>{for(const s of document.querySelectorAll('svg')){const vb=s.viewBox&&s.viewBox.baseVal;if(vb&&vb.width){s.setAttribute('width',vb.width);s.setAttribute('height',vb.height);s.style.maxWidth='none';}}});
await (await p.$('#t')).screenshot({path:out});await b.close();})();
