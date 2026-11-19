// usage: node pair.js <outdir> <scale> <name.svg>... : renders son-pre/<n> | son-head/<n> side by side
const { chromium } = require("playwright"); const fs=require("fs"),path=require("path");
(async()=>{const [o,scale,...f]=process.argv.slice(2);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:parseFloat(scale)});
const V=path.dirname(o);
for(const n of f){const a=fs.readFileSync(path.join(V,"son-pre",n),"utf8").replace(/flowmaid-/g,"pre-fm-"),h=fs.readFileSync(path.join(V,"son-head",n),"utf8").replace(/flowmaid-/g,"head-fm-");
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-flex;gap:24px;padding:8px;align-items:flex-start"><div><div style="font:14px sans-serif">PRE 8876984</div>${a}</div><div><div style="font:14px sans-serif">HEAD c9eb06b</div>${h}</div></div></body></html>`);
await (await p.$("#w")).screenshot({path:path.join(o,n.replace(/\.svg$/,".pair.png"))});}await b.close();})();
