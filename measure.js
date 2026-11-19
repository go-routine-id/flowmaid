const { chromium } = require('playwright');
const fs=require('fs');
(async()=>{
 const b=await chromium.launch(); const p=await b.newPage();
 const svg=fs.readFileSync(process.argv[2],'utf8');
 await p.setContent(`<html><body style="margin:0">${svg}</body></html>`);
 const r=await p.evaluate(()=>{
   const out=[]; const texts=[...document.querySelectorAll('text')];
   for(const t of texts){ const sh=t.previousElementSibling; const a=t.getBBox(), s=sh.getBBox();
     out.push([t.textContent, +(a.x-s.x).toFixed(1), +((s.x+s.width)-(a.x+a.width)).toFixed(1), +a.width.toFixed(1), +s.width.toFixed(1)]); }
   return out; });
 console.log(r.map(x=>x.join(' | ')).join('\n')); await b.close();
})();
