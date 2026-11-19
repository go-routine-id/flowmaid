# Per ER label chip: how many relationship lines pass through it (own line = 1; more = a neighbour's line under the chip).
import re, sys
sys.path.insert(0, __file__.rsplit('/',1)[0])
from ercross_lib import parse
for f in sys.argv[1:]:
    s=open(f).read()
    lines=[parse(m) for m in re.findall(r'<path d="(M [^"]*)" fill="none" stroke="[^"]*" stroke-width="1.7"(?: stroke-dasharray="[^"]*")?/>', s)]
    lines=[l for l in lines if len(l)>2 or (len(l)==2 and abs(l[0][0]-l[1][0])+abs(l[0][1]-l[1][1])>30)]
    chips=[tuple(map(float,m)) for m in re.findall(r'<rect x="([-\d.]+)" y="([-\d.]+)" width="([\d.]+)" height="20" rx="4" fill="#ffffff" stroke="#d5d9ec"/>', s)]
    bad=[]
    for (x,y,w) in chips:
        hits=0
        for l in lines:
            # densify
            pts=[]
            for a,b in zip(l,l[1:]):
                for k in range(10): pts.append((a[0]+(b[0]-a[0])*k/10, a[1]+(b[1]-a[1])*k/10))
            if any(x+1<p[0]<x+w-1 and y+1<p[1]<y+19 for p in pts): hits+=1
        if hits!=1: bad.append(((x,y,w),hits))
    print(f.split('/')[-1], 'chips',len(chips), 'bad', bad)
