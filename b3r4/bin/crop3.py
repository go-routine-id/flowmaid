import sys
from PIL import Image
# crop3.py out.png scale name:x0,y0,x1,y1 ... (from <dir>/{pre,now}/<name>.{static,routed}.png in cwd)
out=sys.argv[1]; sc=float(sys.argv[2]); rows=[]
for a in sys.argv[3:]:
    n,b=a.split(':'); box=tuple(int(v) for v in b.split(','))
    rows.append([Image.open(p).crop(box) for p in ['pre/%s.static.png'%n,'now/%s.static.png'%n,'now/%s.routed.png'%n]])
W=max(sum(i.size[0]+10 for i in r) for r in rows); H=sum(r[0].size[1]+10 for r in rows)
o=Image.new('RGB',(W,H),'white'); y=0
for r in rows:
    x=0
    for i in r: o.paste(i,(x,y)); x+=i.size[0]+10
    y+=r[0].size[1]+10
o=o.resize((int(o.size[0]*sc),int(o.size[1]*sc))); o.save(out)
