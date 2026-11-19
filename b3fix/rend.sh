#!/usr/bin/env bash
# rend.sh <repo> <outdir> <indir>   -> <outdir>/<name>.{static,routed}.png via quality --svg
set -euo pipefail
R=$1; O=$2; I=$3; mkdir -p $O
(cd $R && cargo build --release -q --example quality) 
$R/target/release/examples/quality $I --svg $O > $O/q.tsv 2>/dev/null
NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs"),path=require("path");
(async()=>{const o=process.argv[1];const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1.25});
for(const f of fs.readdirSync(o)){ if(!f.endsWith(".svg")) continue; const s=path.join(o,f);
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:8px">${fs.readFileSync(s,"utf8")}</div></body></html>`);
await (await p.$("#w")).screenshot({path:s.replace(/\.svg$/,".png")});}await b.close();})();' $O
