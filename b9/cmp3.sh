#!/usr/bin/env bash
# usage: cmp3.sh <outdir> <file.mmd>...  -> <outdir>/<name>.three.png (mermaid | pre | now)
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
O=$1; shift; mkdir -p $O
for f in "$@"; do n=$(basename $f .mmd); $S/b9/wt-pre-target/release/flowmaid $f > $O/$n.pre.svg 2>/dev/null; /home/user/flowmaid/target/release/flowmaid $f > $O/$n.now.svg 2>/dev/null; done
NODE_PATH=$(npm root -g) timeout 600 node $S/tools/mmrender.js $O "$@" >/dev/null
NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs"),path=require("path");
(async()=>{const [o,...f]=process.argv.slice(1);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1});
for(const x of f){const n=path.basename(x).replace(/\.mmd$/,"");
 for (const k of ["pre","now"]) {const s=path.join(o,n+"."+k+".svg"); if(!fs.existsSync(s)||!fs.statSync(s).size) continue;
 await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:8px">${fs.readFileSync(s,"utf8")}</div></body></html>`);
 await (await p.$("#w")).screenshot({path:path.join(o,n+"."+k+".png")});}}
await b.close();})();' $O "$@"
python3 - "$O" "$@" <<'PY'
import sys,os
from PIL import Image
o=sys.argv[1]
for f in sys.argv[2:]:
    n=os.path.basename(f)[:-4]
    ps=[os.path.join(o,n+s) for s in ('.mermaid.png','.pre.png','.now.png')]
    ims=[Image.open(p) for p in ps if os.path.exists(p)]
    if not ims: continue
    h=max(i.height for i in ims); w=sum(i.width for i in ims)+20*len(ims)
    c=Image.new('RGB',(w,h),'white'); x=0
    for i in ims: c.paste(i,(x,0)); x+=i.width+20
    c.save(os.path.join(o,n+'.three.png'))
PY
