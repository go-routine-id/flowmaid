#!/usr/bin/env bash
# side.sh <tag> names... : render fix3/<name>.<tag>.svg -> png, and compose mermaid | head | tag
set -euo pipefail
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
tag=$1; shift
W=$S/wt/a3-state
mkdir -p $S/fix3/png
for n in "$@"; do
  [ -f $S/fix3/$n.head.svg ] || (cd $W && git stash -q && cargo build --release -q --bin flowmaid && ./target/release/flowmaid $S/verify/a3-state/in/$n.mmd > $S/fix3/$n.head.svg; git stash pop -q; cargo build --release -q --bin flowmaid)
done
NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs"),path=require("path");
(async()=>{const [d,tag,...f]=process.argv.slice(1);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1.5});
for(const n of f){for(const t of ["head",tag]){const s=path.join(d,n+"."+t+".svg");
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:8px">${fs.readFileSync(s,"utf8")}</div></body></html>`);
await (await p.$("#w")).screenshot({path:path.join(d,"png",n+"."+t+".png")});}}await b.close();})();' $S/fix3 $tag "$@"
python3 - "$S" "$tag" "$@" <<'PY'
import sys
from PIL import Image, ImageDraw
S,tag=sys.argv[1],sys.argv[2]
for n in sys.argv[3:]:
    ims=[Image.open(f"{S}/verify/a3-state/mm/{n}.mermaid.png"),Image.open(f"{S}/fix3/png/{n}.head.png"),Image.open(f"{S}/fix3/png/{n}.{tag}.png")]
    ims=[i.convert("RGB") for i in ims]
    h=max(i.height for i in ims)+30; w=sum(i.width for i in ims)+40
    c=Image.new("RGB",(w,h),"white"); x=0; dr=ImageDraw.Draw(c)
    for t,i in zip(["mermaid","head",tag],ims):
        dr.text((x+10,5),t,fill="black"); c.paste(i,(x,30)); x+=i.width+20; dr.line([(x-10,0),(x-10,h)],fill="gray")
    c.save(f"{S}/fix3/png/{n}.side.{tag}.png")
PY
