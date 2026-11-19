const { chromium } = require("playwright"); const fs=require("fs"),path=require("path");
(async()=>{const dir=process.argv[2];const b=await chromium.launch();const p=await b.newPage();
for(const f of fs.readdirSync(dir).filter(f=>f.endsWith(".svg"))){
await p.setContent(`<html><body style="margin:0">${fs.readFileSync(path.join(dir,f),"utf8")}</body></html>`);
const r=await p.evaluate(()=>{const out=[];const titles=[...document.querySelectorAll("text[font-weight='bold']")];
const chips=[...document.querySelectorAll("rect[fill='#ffffff'][rx='4']")].map(r=>({x0:+r.getAttribute('x'),y0:+r.getAttribute('y'),x1:+r.getAttribute('x')+ +r.getAttribute('width'),y1:+r.getAttribute('y')+ +r.getAttribute('height'),t:r.nextElementSibling&&r.nextElementSibling.textContent}));
for(const t of titles){const bb=t.getBBox();let x0=Infinity,x1=-Infinity;for(let i=0;i<t.getNumberOfChars();i++){try{const e=t.getExtentOfChar(i);x0=Math.min(x0,e.x);x1=Math.max(x1,e.x+e.width);}catch(e){}}
for(const c of chips){const dx=Math.max(x0-c.x1,0,c.x0-x1),dy=Math.max(bb.y-c.y1,0,c.y0-(bb.y+bb.height));const d=Math.hypot(dx,dy);if(d<6)out.push(`${t.textContent} vs chip '${c.t}' gap=${d.toFixed(1)}`);}}
return out;});
console.log(f, r.length? r.join(" | "): "clear(>=6px)");}
await b.close();})();
