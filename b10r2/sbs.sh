#!/bin/bash
# sbs.sh <out-prefix> <files...>: render pre (c72e56b) | head (40ce229) | now side by side
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
P=$1; shift
mkdir -p $S/b10r2/sb_pre $S/b10r2/sb_head $S/b10r2/sb_now
$S/tools/fmrender.sh $S/wt/b10pre $S/b10r2/sb_pre "$@" >/dev/null 2>&1
$S/tools/fmrender.sh $S/b10r2/head $S/b10r2/sb_head "$@" >/dev/null 2>&1
$S/tools/fmrender.sh /home/user/flowmaid $S/b10r2/sb_now "$@" >/dev/null 2>&1
for f in "$@"; do n=$(basename $f .mmd); python3 -c "
from PIL import Image, ImageDraw
import sys
n=sys.argv[1]; S=sys.argv[2]; out=sys.argv[3]
ims=[Image.open(f'{S}/b10r2/{d}/{n}.flowmaid.png') for d in ['sb_pre','sb_head','sb_now']]
w=sum(i.width for i in ims)+40; h=max(i.height for i in ims)+20
c=Image.new('RGB',(w,h),'white'); x=0; d=ImageDraw.Draw(c)
for lab,i in zip(['pre','head','now'],ims):
    c.paste(i,(x,20)); d.text((x+5,2),lab,fill='red'); x+=i.width+20
c.save(out)
" $n $S $S/b10r2/${P}_$n.png; echo $S/b10r2/${P}_$n.png; done
