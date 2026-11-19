#!/usr/bin/env bash
# svg2png.sh <file.svg>... : screenshot each SVG next to itself as .png
NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const f=process.argv.slice(1);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1.5});
for(const s of f){if(!fs.existsSync(s)||!fs.statSync(s).size)continue;
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:8px">${fs.readFileSync(s,"utf8")}</div></body></html>`);
await (await p.$("#w")).screenshot({path:s.replace(/\.svg$/,".png")});}await b.close();})();' "$@"
