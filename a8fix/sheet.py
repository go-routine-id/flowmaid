import sys
from PIL import Image
out, cols, *paths = sys.argv[1:]
cols=int(cols)
ims=[Image.open(p) for p in paths]
rows=[ims[i:i+cols] for i in range(0,len(ims),cols)]
W=max(sum(i.width for i in r)+20*len(r) for r in rows); H=sum(max(i.height for i in r)+20 for r in rows)
o=Image.new('RGB',(W,H),'white'); y=0
for r in rows:
    x=0
    for i in r: o.paste(i,(x,y)); x+=i.width+20
    y+=max(i.height for i in r)+20
o.thumbnail((2400,2400)); o.save(out); print(o.size)
