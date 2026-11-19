# Structural proof: an after SVG is its before SVG with the shift moved from the
# coordinates into the viewBox origin. usage: python3 structcmp.py before after list
import re,sys
A,B,L=sys.argv[1:4]
num=re.compile(r'-?\d+(?:\.\d+)?')
bad=0; worst=0.0; n=0
for name in open(L).read().split():
    a=open(f'{A}/{name}').read(); b=open(f'{B}/{name}').read()
    vb=re.search(r'viewBox="(\S+) (\S+) (\S+) (\S+)"',b)
    ox,oy=float(vb.group(1)),float(vb.group(2))
    # undo the two header changes: viewBox origin and the background rect's x/y
    b2=b.replace(vb.group(0),'viewBox="0 0 %s %s"'%(vb.group(3),vb.group(4)),1)
    b2=re.sub(r'<rect x="[^"]*" y="[^"]*" (width="[^"]*" height="[^"]*" fill="[^"]*"/>)',r'<rect \1',b2,count=1)
    sa=num.sub('#',a); sb=num.sub('#',b2)
    if sa!=sb:
        bad+=1; print('SKELETON',name); continue
    na=[float(x) for x in num.findall(a)]; nb=[float(x) for x in num.findall(b2)]
    ok=True
    for x,y in zip(na,nb):
        d=min(abs(x-y), abs(x-(y-ox)), abs(x-(y-oy)))
        worst=max(worst,d)
        if d>0.1501: ok=False
    n+=1
    if not ok: bad+=1; print('NUMBERS',name)
print(f'checked {n} pairs, failures {bad}, worst residual {worst:.3f}px')
