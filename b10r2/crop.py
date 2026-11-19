import sys
from PIL import Image
# crop.py name x0 y0 x1 y1 out  (coords in single-panel png px) -> pre|head|now crops side by side, scaled up
n,x0,y0,x1,y1,out=sys.argv[1],*map(int,sys.argv[2:6]),sys.argv[6]
S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b10r2'
cs=[Image.open(f'{S}/{d}/{n}.flowmaid.png').crop((x0,y0,x1,y1)) for d in ['sb_pre','sb_head','sb_now']]
w=sum(c.width for c in cs)+20; h=max(c.height for c in cs)
o=Image.new('RGB',(w,h),'red'); x=0
for c in cs: o.paste(c,(x,0)); x+=c.width+10
o.save(out)
