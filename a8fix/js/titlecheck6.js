const { chromium } = require("playwright"); const fs=require("fs"),path=require("path");
(async()=>{const dir=process.argv[2];const b=await chromium.launch();const p=await b.newPage();
let files=fs.readdirSync(dir).filter(f=>f.endsWith(".svg"));let tot=0,hits=[];
for(const f of files){await p.setContent(`<html><body style="margin:0">${fs.readFileSync(path.join(dir,f),"utf8")}</body></html>`);
const r=await p.evaluate(()=>{const out=[];const titles=[...document.querySelectorAll("text[font-weight='bold']")];
const polys=[...document.querySelectorAll("polyline")].map(pl=>pl.getAttribute("points").trim().split(/\s+/).map(s=>s.split(",").map(Number)));
for(const t of titles){const bb=t.getBBox();
// tight glyph box: use extent of chars
let x0=Infinity,x1=-Infinity;for(let i=0;i<t.getNumberOfChars();i++){try{const e=t.getExtentOfChar(i);x0=Math.min(x0,e.x);x1=Math.max(x1,e.x+e.width);}catch(e){}}
const y0=bb.y+bb.height*0.2,y1=bb.y+bb.height*0.85;
for(const pts of polys){for(let k=0;k+1<pts.length;k++){const [ax,ay]=pts[k],[bx,by]=pts[k+1];
// distance from segment to rect
const n=Math.max(1,Math.ceil((Math.abs(bx-ax)+Math.abs(by-ay))/0.5));let md=Infinity;
for(let j=0;j<=n;j++){const u=j/n,x=ax+(bx-ax)*u,y=ay+(by-ay)*u;const dx=Math.max(x0-x,0,x-x1),dy=Math.max(y0-y,0,y-y1);md=Math.min(md,Math.hypot(dx,dy));}
if(md<6.0){out.push({title:t.textContent,md:+md.toFixed(2),x1:+x1.toFixed(1)});break;}}}}
return out;});
if(r.length){tot+=r.length;hits.push(f+": "+r.map(h=>`${h.title} d=${h.md}`).join("; "));}}
console.log(`files ${files.length}, title-near-miss(<6px, incl. touching) ${tot} in ${hits.length} files`);hits.slice(0,40).forEach(h=>console.log("  "+h));await b.close();})();
