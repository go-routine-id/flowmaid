const {chromium}=require('playwright');
(async()=>{const b=await chromium.launch();const p=await b.newPage({viewport:{width:960,height:1400}});
await p.goto('file://'+process.argv[2]);
for (const [id,name] of [['flowmaid','top'],['flowcharts','flow'],['styling-edges-linkstyle','link'],['how-it-works','how']]){
 await p.evaluate(i=>{const h=[...document.querySelectorAll('h1,h2,h3')].find(e=>e.textContent.toLowerCase().replace(/[^\w\- ]/g,'').replace(/ /g,'-')===i); if(h) h.scrollIntoView();},id);
 await p.screenshot({path:process.argv[3]+'/'+name+'.png'});}
await b.close();})();
