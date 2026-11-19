// usage: node pair2.js <out.png> <scale> <a.svg> <b.svg> [c.svg]: whole svgs side by side, labelled by file dir
const { chromium } = require("playwright"); const fs=require("fs"),path=require("path");
(async()=>{const [o,scale,...fs_]=process.argv.slice(2);
const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:parseFloat(scale)});
const parts=fs_.map((f,i)=>`<div><div style="font:13px sans-serif">${path.basename(path.dirname(f))}</div>${fs.readFileSync(f,"utf8").replace(/flowmaid-/g,"v"+i+"-fm-")}</div>`).join("");
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-flex;gap:24px;padding:8px;align-items:flex-start">${parts}</div></body></html>`);
await (await p.$("#w")).screenshot({path:o});await b.close();})();
