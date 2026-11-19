#!/usr/bin/env bash
# pair.sh <outdir> <binA> <binB> names... : renders name with A and B and stacks them (A on top)
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
O=$1; A=$2; B=$3; shift 3; mkdir -p $O
for n in "$@"; do f=$S/b8b/bulkin/$n.mmd; [ -f $f ] || f=$n; b=$(basename $f .mmd); $A $f > $O/$b.A.svg 2>/dev/null; $B $f > $O/$b.B.svg 2>/dev/null; done
NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs"),path=require("path");
(async()=>{const [o,...f]=process.argv.slice(1);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1,viewport:{width:4000,height:3000}});
for(const x of f){const n=path.basename(x).replace(/\.mmd$/,"");
const a=fs.readFileSync(path.join(o,n+".A.svg"),"utf8"), bb=fs.readFileSync(path.join(o,n+".B.svg"),"utf8");
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:8px"><div style="font:14px sans-serif;color:#c00">A</div>${a}<hr><div style="font:14px sans-serif;color:#c00">B</div>${bb}</div></body></html>`);
await (await p.$("#w")).screenshot({path:path.join(o,n+".pair.png")});}await b.close();})();' $O "$@"
