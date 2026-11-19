# crop.py <in.svg> <out.svg> x y w h: same drawing, viewBox cropped to the rectangle (scale 1)
import sys,re
s=open(sys.argv[1]).read(); x,y,w,h=sys.argv[3:7]
def root(m):
    t=m.group(0)
    t=re.sub(r'viewBox="[^"]*"',f'viewBox="{x} {y} {w} {h}"',t,count=1)
    t=re.sub(r'\swidth="[^"]*"',f' width="{w}"',t,count=1)
    t=re.sub(r'\sheight="[^"]*"',f' height="{h}"',t,count=1)
    return t
s=re.sub(r'<svg[^>]*>',root,s,count=1)
open(sys.argv[2],'w').write(s)
