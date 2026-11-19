# Per ER label chip k (edge k): which OTHER relationship lines pass under it.
import re, sys
sys.path.insert(0, __file__.rsplit('/',1)[0])
from ercross_lib import parse
tot=0
for f in sys.argv[1:]:
    s=open(f).read()
    lines=[parse(m) for m in re.findall(r'<path d="(M [^"]*)" fill="none" stroke="[^"]*" stroke-width="1.7"(?: stroke-dasharray="[^"]*")?/>', s)]
    lines=[l for l in lines if len(l)>2 or (len(l)==2 and abs(l[0][0]-l[1][0])+abs(l[0][1]-l[1][1])>30)]
    chips=[tuple(map(float,m)) for m in re.findall(r'<rect x="([-\d.]+)" y="([-\d.]+)" width="([\d.]+)" height="20" rx="4" fill="#ffffff" stroke="#d5d9ec"/>', s)]
    if len(chips)!=len(lines):
        print(f.split('/')[-1],'skip',len(chips),len(lines)); continue
    bad=[]
    for k,(x,y,w) in enumerate(chips):
        for j,l in enumerate(lines):
            if j==k: continue
            pts=[]
            for a,b in zip(l,l[1:]):
                for t in range(10): pts.append((a[0]+(b[0]-a[0])*t/10, a[1]+(b[1]-a[1])*t/10))
            if any(x-1<p[0]<x+w+1 and y-1<p[1]<y+21 for p in pts): bad.append((k,j))
    tot+=len(bad)
    if bad: print(f.split('/')[-1], 'chip-on-other-line', bad)
print('total',tot)
