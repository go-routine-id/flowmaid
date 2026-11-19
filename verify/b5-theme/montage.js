const { chromium } = require('playwright'); const fs=require('fs'),path=require('path');
(async()=>{const [dir,out,themes,...names]=process.argv.slice(2);const ts=themes.split(',');
const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1});
for(const n of names){let html='<html><body style="margin:0;background:#888"><div id=w style="display:inline-flex;gap:12px;padding:8px;align-items:flex-start">';
for(const t of ts){const f=path.join(dir,`${n}.${t}.svg`);if(!fs.existsSync(f))continue;html+=`<div><div style="font:bold 14px sans-serif;color:#fff">${t}</div>${fs.readFileSync(f,'utf8')}</div>`;}
html+='</div></body></html>';await p.setContent(html);await (await p.$('#w')).screenshot({path:path.join(out,`${n}.themes.png`)});}
await b.close();})();
