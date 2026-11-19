import sys
from PIL import Image
# crop.py in.png out.png x0 y0 x1 y1 [scale]
a=sys.argv
im=Image.open(a[1]); x0,y0,x1,y1=map(int,a[3:7]); s=float(a[7]) if len(a)>7 else 1.0
c=im.crop((x0,y0,x1,y1))
if s!=1.0: c=c.resize((int(c.width*s),int(c.height*s)),Image.LANCZOS)
c.save(a[2])
print(im.size, c.size)
