// node three.js <out.png> <title=file.(svg|png)>...   side by side, images or svgs
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [out,...cols]=process.argv.slice(2);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1.0});
let html='<html><body style="margin:0;background:#fff;font:14px sans-serif"><div id=w style="display:inline-flex;gap:16px;padding:8px;align-items:flex-start">';
for(const c of cols){const i=c.indexOf("=");const f=c.slice(i+1);let body;
 if(f.endsWith(".png")){body=`<img src="data:image/png;base64,${fs.readFileSync(f).toString("base64")}" style="max-width:1400px">`;}
 else body=fs.readFileSync(f,"utf8");
 html+=`<div><div>${c.slice(0,i)}</div>${body}</div>`;}
html+='</div></body></html>';await p.setContent(html);await (await p.$("#w")).screenshot({path:out});await b.close();})();
