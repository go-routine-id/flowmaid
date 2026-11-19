import sys
from PIL import Image
# crop.py in.png out.png x0 y0 x1 y1 (fractions) [maxw]
im=Image.open(sys.argv[1]); W,H=im.size
x0,y0,x1,y1=[float(v) for v in sys.argv[3:7]]
mw=int(sys.argv[7]) if len(sys.argv)>7 else 1600
c=im.crop((int(x0*W),int(y0*H),int(x1*W),int(y1*H)))
if c.size[0]>mw: c=c.resize((mw,int(c.size[1]*mw/c.size[0])))
c.save(sys.argv[2])
print(c.size)
