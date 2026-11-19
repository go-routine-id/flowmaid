#!/bin/bash
# zoom.sh <file-hash-prefix> <node-id> [half=150]: crops pre|head|now around the node
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
F=$(ls $S/b8r1/bulkst/$1*.mmd 2>/dev/null || ls $1); N=$2; H=${3:-150}
n=$(basename $F .mmd)
[ -f $S/b10r2/sb_now/$n.flowmaid.png ] && [ -z "$FRESH" ] || $S/b10r2/sbs.sh s $F >/dev/null
xy=$(python3 -c "
import re,sys
s=open('$S/b10r2/sb_now/$n.flowmaid.svg').read()
m=re.search(r'viewBox=\"([-\d.]+) ([-\d.]+)',s); ox,oy=float(m.group(1)),float(m.group(2))
print(ox,oy)")
pos=$(D=1 $S/b10r2/probe/target/release/probe $F | grep "^N $N " | awk '{print $3, $4}')
python3 - $n $pos $xy $H <<'PY'
import sys
from PIL import Image
n,x,y,ox,oy,H=sys.argv[1],float(sys.argv[2]),float(sys.argv[3]),float(sys.argv[4]),float(sys.argv[5]),float(sys.argv[6])
S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b10r2'
cs=[]
for d in ['sb_pre','sb_head','sb_now']:
    im=Image.open(f'{S}/{d}/{n}.flowmaid.png')
    import re
    s=open(f'{S}/{d}/{n}.flowmaid.svg').read()
    m=re.search(r'viewBox="([-\d.]+) ([-\d.]+)',s); ox,oy=float(m.group(1)),float(m.group(2))
    px,py=(x-ox)*1.5+12,(y-oy)*1.5+12
    cs.append(im.crop((int(px-H*1.5),int(py-H*1.5),int(px+H*1.5),int(py+H*1.5))))
w=sum(c.width for c in cs)+20; h=max(c.height for c in cs)
o=Image.new('RGB',(w,h),'red'); xx=0
for c in cs: o.paste(c,(xx,0)); xx+=c.width+10
o.save(f'{S}/z_{n[:8]}.png'); print(f'{S}/z_{n[:8]}.png')
PY
