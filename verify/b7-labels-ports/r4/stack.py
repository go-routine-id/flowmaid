import sys
from PIL import Image, ImageDraw
out=sys.argv[1]; items=sys.argv[2:]
ims=[]
for it in items:
    t,p=it.split('=',1)
    try: im=Image.open(p).convert('RGB')
    except Exception: im=Image.new('RGB',(200,40),'white')
    ims.append((t,im))
W=max(i.width for _,i in ims); H=sum(i.height+24 for _,i in ims)
c=Image.new('RGB',(W,H),'white'); d=ImageDraw.Draw(c); y=0
for t,i in ims:
    d.text((4,y+4),t,fill='black'); d.line([(0,y),(W,y)],fill='#888'); c.paste(i,(0,y+22)); y+=i.height+24
c.save(out)
