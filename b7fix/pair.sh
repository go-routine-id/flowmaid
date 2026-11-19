#!/usr/bin/env bash
# pair.sh <out.png> <file.mmd> : head0 | now side by side
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
O=$1; F=$2; T=$(mktemp -d -p $S/b7fix)
${REF:-$S/b7fix/head0/target/release/flowmaid} $F > $T/a.svg; /home/user/flowmaid/target/release/flowmaid $F > $T/b.svg
cd $T && NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:Number(process.argv[2]||1)});
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:flex;gap:30px;padding:8px;align-items:flex-start">${fs.readFileSync("a.svg","utf8")}<div style="width:3px;background:#f00;align-self:stretch"></div>${fs.readFileSync("b.svg","utf8")}</div></body></html>`);
await (await p.$("#w")).screenshot({path:process.argv[1]});await b.close();})();' $O ${SCALE:-1}
rm -rf $T
