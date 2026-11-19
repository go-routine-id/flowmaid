#!/usr/bin/env bash
# crop.sh <svg> <x> <y> <w> <h> <out.png> [scale]
NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [s,x,y,w,h,o,sc]=process.argv.slice(1);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:parseFloat(sc||"1")});
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:0px">${fs.readFileSync(s,"utf8")}</div></body></html>`);
await p.setViewportSize({width: 4000, height: 3000});
await p.screenshot({path:o, clip:{x:+x,y:+y,width:+w,height:+h}});await b.close();})();' "$@"
