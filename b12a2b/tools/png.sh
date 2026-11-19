#!/usr/bin/env bash
# png.sh <scale> <file.svg>...  -> file.png beside each
sc=$1; shift
NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [sc,...f]=process.argv.slice(1);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:+sc});
for(const s of f){await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:4px">${fs.readFileSync(s,"utf8")}</div></body></html>`);
await (await p.$("#w")).screenshot({path:s.replace(/\.svg$/,".png")});}await b.close();})();' $sc "$@"
