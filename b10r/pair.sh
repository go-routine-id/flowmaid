#!/usr/bin/env bash
# pair.sh <outdir> <files.mmd...> : render pre (wt/b10pre) and now (/home/user/flowmaid) side by side -> <out>/<name>.pair.png
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
O=$1; shift; mkdir -p $O
for f in "$@"; do n=$(basename $f .mmd)
  $S/wt/b10pre/target/release/flowmaid $f > $O/$n.pre.svg 2>/dev/null
  /home/user/flowmaid/target/release/flowmaid $f > $O/$n.now.svg 2>/dev/null
done
NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs"),path=require("path");
(async()=>{const [o,...f]=process.argv.slice(1);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1.5});
for(const x of f){const n=path.basename(x).replace(/\.mmd$/,"");
const a=fs.readFileSync(path.join(o,n+".pre.svg"),"utf8"), c=fs.readFileSync(path.join(o,n+".now.svg"),"utf8");
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-flex;gap:24px;padding:8px;align-items:flex-start"><div><div style="font:14px sans-serif">pre</div>${a}</div><div><div style="font:14px sans-serif">now</div>${c}</div></div></body></html>`);
await (await p.$("#w")).screenshot({path:path.join(o,n+".pair.png")});}await b.close();})();' $O "$@"
