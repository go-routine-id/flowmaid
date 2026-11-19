const { chromium } = require('playwright');
const path=require('path');
const mm = path.join(__dirname, '../mm/node_modules/mermaid/dist/mermaid.min.js');
(async()=>{
 const b=await chromium.launch(); const p=await b.newPage();
 await p.setContent('<html><body><div id=c></div></body></html>');
 await p.addScriptTag({path:mm});
 await p.evaluate(()=>mermaid.initialize({startOnLoad:false}));
 for (const src of ['gitGraph TB:\n commit id: "init" tag: "v0.1-alpha"\n branch feat\n commit id: "a"','gitGraph LR:\n commit']) {
  try { const svg = await p.evaluate(async s=>(await mermaid.render('x'+Math.floor(Math.random()*1e9), s)).svg, src); console.log('OK', src.split('\n')[0], svg.length); }
  catch(e){ console.log('ERR', src.split('\n')[0], String(e).slice(0,200)); }
 }
 await b.close();
})();
