#!/usr/bin/env bash
# usage: svgs2png.sh out.png a.svg b.svg ... -> side by side
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
OUT=$1; shift
NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [o,...f]=process.argv.slice(1);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1});
const html=f.map(x=>`<div style="display:inline-block;vertical-align:top;margin:4px;border:1px solid #ccc">${fs.readFileSync(x,"utf8")}</div>`).join("");
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;white-space:nowrap">${html}</div></body></html>`);
await (await p.$("#w")).screenshot({path:o});await b.close();})();' $OUT "$@"
