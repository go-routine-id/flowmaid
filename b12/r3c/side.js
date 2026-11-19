// node side.js <out.png> <title=file.svg>...
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [out,...cols]=process.argv.slice(2);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1.2});
let html='<html><body style="margin:0;background:#fff;font:14px sans-serif"><div id=w style="display:inline-flex;gap:16px;padding:8px">';
for(const c of cols){const i=c.indexOf("=");html+=`<div><div>${c.slice(0,i)}</div>${fs.readFileSync(c.slice(i+1),"utf8")}</div>`;}
html+='</div></body></html>';await p.setContent(html);await (await p.$("#w")).screenshot({path:out});await b.close();})();
