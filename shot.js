const { chromium } = require('playwright');
(async()=>{const b=await chromium.launch();const p=await b.newPage({viewport:{width:1280,height:900}});
await p.goto('file://'+process.argv[2]);await p.waitForTimeout(800);
await p.screenshot({path:process.argv[3],fullPage:false});
await p.evaluate(()=>document.getElementById('d-demo').scrollIntoView());await p.waitForTimeout(400);
await p.screenshot({path:process.argv[4]});
const w=await p.evaluate(()=>document.documentElement.scrollWidth);console.log('scrollWidth',w);await b.close();})();
