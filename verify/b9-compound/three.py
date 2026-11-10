import sys,os
from PIL import Image, ImageDraw
V=sys.argv[1]
for n in sys.argv[2:]:
    ps=[f'{V}/mm/{n}.mermaid.png', f'{V}/pre-out/{n}.flowmaid.png', f'{V}/now/{n}.flowmaid.png']
    ims=[Image.open(p) if os.path.exists(p) else Image.new('RGB',(100,100),'pink') for p in ps]
    h=max(i.height for i in ims)+24; w=sum(i.width for i in ims)+60
    c=Image.new('RGB',(w,h),'white'); x=0; d=ImageDraw.Draw(c)
    for lab,i in zip(['mermaid','pre','now'],ims):
        d.text((x+4,2),lab,fill='red'); c.paste(i,(x,24)); 
        d.line([(x+i.width+10,0),(x+i.width+10,h)],fill='gray'); x+=i.width+20
    c.save(f'{V}/three-{n}.png')
