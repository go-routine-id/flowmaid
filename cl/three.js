// three.js <out.png> <mmDir> <preDir> <nowDir> <name>... : contact sheet mermaid | before | now (each scaled to <=W px wide)
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [out,mm,pre,now,...names]=process.argv.slice(2);const W=+process.env.W||520;
const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1});
const rd=f=>fs.existsSync(f)&&fs.statSync(f).size?fs.readFileSync(f,"utf8"):"(none)";
let html="<html><body style=\"margin:0;background:#fff;font:13px sans-serif\"><table>";
for(const n of names){html+=`<tr><td style="vertical-align:top;border:1px solid #ccc;padding:4px">${n} mermaid<br>${rd(`${mm}/${n}.mermaid.svg`)}</td><td style="vertical-align:top;border:1px solid #ccc;padding:4px">before<br>${rd(`${pre}/${n}.svg`)}</td><td style="vertical-align:top;border:1px solid #ccc;padding:4px">now<br>${rd(`${now}/${n}.svg`)}</td></tr>`;}
html+="</table></body></html>";await p.setContent(html);
await p.evaluate((W)=>{document.querySelectorAll("svg").forEach(e=>{const vb=e.viewBox.baseVal;if(vb&&vb.width){const k=Math.min(1,W/vb.width);e.setAttribute("width",vb.width*k);e.setAttribute("height",vb.height*k);e.style.maxWidth="none";}});},W);
await (await p.$("table")).screenshot({path:out});await b.close();})();
