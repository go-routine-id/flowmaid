import sys
from PIL import Image
# crop.py in.png out.png x0 y0 x1 y1 [scale]  (scene coords; png at 1.5x + 8px pad)
a=sys.argv; im=Image.open(a[1]); x0,y0,x1,y1=map(float,a[3:7]); z=float(a[7]) if len(a)>7 else 2
f=lambda v:int(v*1.5+12)
c=im.crop((f(x0),f(y0),f(x1),f(y1))); c=c.resize((int(c.width*z/1.5),int(c.height*z/1.5)),Image.LANCZOS); c.save(a[2])
