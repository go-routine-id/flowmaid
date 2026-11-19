# crop.py in.png out.png x0 y0 x1 y1 [scale]  (coords in displayed px of the png)
import sys
from PIL import Image
a=sys.argv
im=Image.open(a[1]); x0,y0,x1,y1=map(int,a[3:7]); sc=float(a[7]) if len(a)>7 else 3
c=im.crop((x0,y0,x1,y1)); c=c.resize((int((x1-x0)*sc),int((y1-y0)*sc)),Image.LANCZOS); c.save(a[2])
