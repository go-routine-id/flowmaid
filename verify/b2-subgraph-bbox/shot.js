const { chromium } = require("playwright"); const fs=require("fs"),path=require("path");
(async()=>{const [dir]=process.argv.slice(2);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1});
for(const f of fs.readdirSync(dir).filter(x=>x.endsWith(".svg"))){const s=path.join(dir,f);
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:0">${fs.readFileSync(s,"utf8")}</div></body></html>`);
await p.evaluate(() => { const s = document.querySelector('#w svg'); const vb = s.viewBox.baseVal; s.setAttribute('width', vb.width); s.setAttribute('height', vb.height); });
await (await p.$("#w svg")).screenshot({path:s.replace(/\.svg$/,".png")});}await b.close();})();
