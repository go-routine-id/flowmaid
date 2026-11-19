#!/usr/bin/env bash
# cmp.sh <outdir> <file.mmd>... : static+routed svg/png for pre, head, now; stacked compare png per file
set -euo pipefail
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
O=$1; shift; mkdir -p $O/in
for f in "$@"; do cp $f $O/in/; done
(cd /home/user/flowmaid && cargo build --release -q --example quality 2>&1 | tail -3)
for v in pre head now; do
  case $v in pre) Q=$S/b3r3/tgt_pre/release/examples/quality;; head) Q=$S/b3r3/tgt_head/release/examples/quality;; now) Q=/home/user/flowmaid/target/release/examples/quality;; esac
  $Q $O/in --svg $O/$v > $O/$v.tsv 2>/dev/null
done
NODE_PATH=$(npm root -g) node $S/b3r3/svg2png.js $O/pre/*.svg $O/head/*.svg $O/now/*.svg
python3 - "$O" "$@" <<'PY'
import sys,os
from PIL import Image, ImageDraw
O=sys.argv[1]
for f in sys.argv[2:]:
    n=os.path.basename(f)[:-4]
    for mode in ['static','routed']:
        ps=[f"{O}/{v}/{n}.{mode}.png" for v in ['pre','head','now']]
        ims=[Image.open(p).convert('RGB') for p in ps]
        W=max(i.size[0] for i in ims); H=sum(i.size[1] for i in ims)+30*3
        c=Image.new('RGB',(W,H),'white'); d=ImageDraw.Draw(c); y=0
        for v,i in zip(['pre','head','now'],ims):
            d.text((5,y+5),f"{n} {mode} {v}",fill='red'); c.paste(i,(0,y+25)); y+=i.size[1]+30
        c.save(f"{O}/{n}.{mode}.cmp.png")
PY
echo done $O
