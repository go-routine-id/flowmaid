const { chromium } = require('playwright'); const fs=require('fs'),path=require('path');
(async()=>{const [out,...fs_]=process.argv.slice(2);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1.5});
for(const f of fs_){await p.setContent(`<html><body style="margin:0;background:#888"><div id=w style="display:inline-block;padding:8px">${fs.readFileSync(f,'utf8')}</div></body></html>`);
await (await p.$('#w')).screenshot({path:path.join(out,path.basename(f).replace(/\.svg$/,'.png'))});}await b.close();})();
