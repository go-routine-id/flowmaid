import sys,re
src,out,x,y,w,h=sys.argv[1],sys.argv[2],*map(float,sys.argv[3:7])
s=open(src).read()
s=re.sub(r'viewBox="[^"]*"',f'viewBox="{x} {y} {w} {h}"',s,count=1)
s=re.sub(r'<svg([^>]*?) width="[^"]*"',rf'<svg\1 width="{w*1.0}"',s,count=1)
s=re.sub(r'<svg([^>]*?) height="[^"]*"',rf'<svg\1 height="{h*1.0}"',s,count=1)
open(out,'w').write(s)
