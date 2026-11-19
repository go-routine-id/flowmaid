const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const b=await chromium.launch();const p=await b.newPage();
for (const f of process.argv.slice(2)) {
  await p.setContent(`<html><body style="margin:0;background:#fff">${fs.readFileSync(f,"utf8")}</body></html>`);
  const r = await p.evaluate(()=>{
    const svg=document.querySelector("svg"); const vb=svg.viewBox.baseVal;
    const texts=[...svg.querySelectorAll("text")].filter(t=>t.getAttribute("aria-hidden")!=="true").map(t=>{const bb=t.getBBox();return {s:t.textContent,x0:bb.x,y0:bb.y,x1:bb.x+bb.width,y1:bb.y+bb.height};});
    const rects=[...svg.querySelectorAll("rect")].slice(1).map(r=>({x0:+r.getAttribute("x"),y0:+r.getAttribute("y"),x1:+r.getAttribute("x")+ +r.getAttribute("width"),y1:+r.getAttribute("y")+ +r.getAttribute("height"),t:(r.querySelector("title")||{}).textContent||""}));
    const ov=(a,b,m=0)=>a.x0<b.x1-m&&b.x0<a.x1-m&&a.y0<b.y1-m&&b.y0<a.y1-m;
    const out=[];
    for(let i=0;i<texts.length;i++)for(let j=i+1;j<texts.length;j++) if(ov(texts[i],texts[j],1)) out.push(`LABEL/LABEL ${texts[i].s} | ${texts[j].s}`);
    for(const t of texts) for(const r of rects){ // shrink glyph bbox vertically (bbox includes line gap)
      const tt={x0:t.x0,x1:t.x1,y0:t.y0+3,y1:t.y1-3};
      if(ov(tt,r,0.5)) { const own = r.t && t.s.startsWith(r.t.split(" (")[0].split("\n")[0].slice(0,6)); out.push(`LABEL/NODE ${t.s} | rect ${r.t||JSON.stringify(r)}${own?" (own?)":""}`);} }
    for(const t of texts) if(t.x0<vb.x-0.5||t.x1>vb.x+vb.width+0.5||t.y0<vb.y-0.5||t.y1>vb.y+vb.height+0.5) out.push(`OUTSIDE ${t.s} ${t.x0.toFixed(0)}..${t.x1.toFixed(0)} vb ${vb.width}`);
    return out;
  });
  console.log(f.split("/").pop()+": "+(r.length?"\n  "+r.join("\n  "):"clean"));
}
await b.close();})();
