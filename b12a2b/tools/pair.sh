#!/usr/bin/env bash
# pair.sh <out.png> <a.png> <b.png>  side by side
NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [o,a,c]=process.argv.slice(1);const b=await chromium.launch();const p=await b.newPage();
const im=x=>"data:image/png;base64,"+fs.readFileSync(x).toString("base64");
await p.setContent(`<html><body style="margin:0;background:#888"><div id=w style="display:inline-flex;gap:8px;align-items:flex-start"><img src="${im(a)}"><img src="${im(c)}"></div></body></html>`);
await p.waitForTimeout(100);await (await p.$("#w")).screenshot({path:o});await b.close();})();' "$@"
