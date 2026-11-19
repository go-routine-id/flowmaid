const { chromium } = require("playwright"); const fs=require("fs"),path=require("path");
(async()=>{const [o,...names]=process.argv.slice(2);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1.3});
for(const n of names){const cols=[["base (static)",n+".head.svg.base.svg"],["route_partial pre",n+".pre.svg"],["route_partial head",n+".head.svg"]];
let html='<html><body style="margin:0;background:#fff;font:14px sans-serif"><div id=w style="display:inline-flex;gap:16px;padding:8px">';
for(const [t,f] of cols){html+=`<div><div>${t}</div>${fs.readFileSync(path.join(o,f),"utf8")}</div>`;}
html+='</div></body></html>';await p.setContent(html);await (await p.$("#w")).screenshot({path:path.join(o,n+".side.png")});}
await b.close();})();
