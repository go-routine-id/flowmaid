import sys
from PIL import Image
n=sys.argv[1]
ims=[Image.open(p) for p in (f'mm/{n}.mermaid.png', f'fm0/{n}.flowmaid.png', f'fm1/{n}.flowmaid.png')]
h=max(i.height for i in ims); w=sum(i.width for i in ims)+40
c=Image.new('RGB',(w,h),'white'); x=0
for i in ims: c.paste(i,(x,0)); x+=i.width+20
c.save(f'three-{n}.png')
