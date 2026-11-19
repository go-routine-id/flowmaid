const { chromium } = require("playwright");
(async()=>{const strs=JSON.parse(process.argv[2]);const b=await chromium.launch();const p=await b.newPage();
await p.setContent(`<svg xmlns="http://www.w3.org/2000/svg" width="900" height="200" font-family="Helvetica, Arial, sans-serif" font-size="14"><g id=g></g></svg>`);
const r=await p.evaluate((strs)=>{const g=document.getElementById('g');return strs.map(s=>{
 const out={s};for(const bold of [false,true]){const t=document.createElementNS('http://www.w3.org/2000/svg','text');t.setAttribute('x','10');t.setAttribute('y','50');if(bold)t.setAttribute('font-weight','bold');t.textContent=s;g.appendChild(t);
 let x0=Infinity,x1=-Infinity;for(let i=0;i<t.getNumberOfChars();i++){try{const e=t.getExtentOfChar(i);x0=Math.min(x0,e.x);x1=Math.max(x1,e.x+e.width);}catch(e){}}
 out[bold?'bold':'reg']=+(x1-10).toFixed(2);g.removeChild(t);}return out;});},strs);
for(const x of r)console.log(JSON.stringify(x));await b.close();})();
