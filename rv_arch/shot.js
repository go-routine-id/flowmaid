const { chromium } = require('playwright');
const fs=require('fs');
(async()=>{const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1.5,viewport:{width:1800,height:800}});
for (const f of process.argv.slice(2)){await p.setContent(`<html><body style="margin:0;background:#fff;display:inline-block"><div id=w style="display:inline-block;padding:8px">${fs.readFileSync(f,'utf8')}</div></body></html>`);
await (await p.$('#w')).screenshot({path:f.replace(/\.svg$/,'.png')});}
await b.close();})();
