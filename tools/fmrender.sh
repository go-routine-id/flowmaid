#!/usr/bin/env bash
# Render arbitrary .mmd with a flowmaid repo build -> <out>/<name>.flowmaid.svg/.png ; usage: fmrender.sh <repo> <outDir> <file.mmd>...
set -euo pipefail
R=$1; O=$2; shift 2; mkdir -p $O; (cd $R && cargo build --release -q --bin flowmaid)
for f in "$@"; do n=$(basename $f .mmd); $R/target/release/flowmaid $f > $O/$n.flowmaid.svg 2>$O/$n.err || echo "ERR $n: $(head -c 300 $O/$n.err)"; done
NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs"),path=require("path");
(async()=>{const [o,...f]=process.argv.slice(1);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1.5});
for(const x of f){const n=path.basename(x).replace(/\.mmd$/,"");const s=path.join(o,n+".flowmaid.svg");if(!fs.existsSync(s)||!fs.statSync(s).size)continue;
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:8px">${fs.readFileSync(s,"utf8")}</div></body></html>`);
await (await p.$("#w")).screenshot({path:path.join(o,n+".flowmaid.png")});}await b.close();})();' $O "$@"
echo "rendered to $O"
