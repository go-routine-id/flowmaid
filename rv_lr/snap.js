const { chromium } = require('/opt/node-tools/node_modules/playwright');
const fs=require('fs');
(async()=>{
 const b=await chromium.launch(); const p=await b.newPage({deviceScaleFactor:2, viewport:{width:1400,height:900}});
 const args=process.argv.slice(2);
 for (let i=0;i<args.length;i+=2){
  const svg=fs.readFileSync(args[i],'utf8');
  await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block">${svg}</div></body></html>`);
  await (await p.$('#w')).screenshot({path:args[i+1]});
 }
 await b.close();
})();
