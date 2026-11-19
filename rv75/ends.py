import re,sys,math
svg=open(sys.argv[1]).read()
svg=re.sub(r'<mask .*?</mask>','',svg,flags=re.S)
nodes={}
for m in re.finditer(r'<g><title>([^<]*)</title>\s*<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)"',svg):
    nodes[m.group(1)]=tuple(map(float,m.groups()[1:]))
def lastseg(d):
    toks=re.findall(r'[MLC]|-?[\d.]+',d)
    # get last two distinct points: final point and previous control/point
    nums=[t for t in toks if t not in 'MLC']
    pts=[(float(nums[i]),float(nums[i+1])) for i in range(0,len(nums),2)]
    end=pts[-1]
    for p in reversed(pts[:-1]):
        if math.dist(p,end)>0.5: return p,end,pts[0],pts[1]
res=[]; corner=0; obl=0
for m in re.finditer(r'<path d="([^"]*)"[^>]*><title>([^<]*)</title>',svg):
    d,t=m.groups(); a,b=[s.strip() for s in t.split('→')]
    prev,end,start,s2=lastseg(d)
    x,y,w,h=nodes.get(b) or nodes["external api"]
    # which side
    ds={'L':abs(end[0]-x),'R':abs(end[0]-x-w),'T':abs(end[1]-y),'B':abs(end[1]-y-h)}
    side=min(ds,key=ds.get)
    n={'L':(1,0),'R':(-1,0),'T':(0,1),'B':(0,-1)}[side]
    v=(end[0]-prev[0],end[1]-prev[1]); L=math.hypot(*v); v=(v[0]/L,v[1]/L)
    ang=math.degrees(math.acos(max(-1,min(1,v[0]*n[0]+v[1]*n[1]))))
    # distance from corner along side
    if side in 'LR': dc=min(end[1]-y, y+h-end[1])
    else: dc=min(end[0]-x, x+w-end[0])
    res.append((round(ang),side,round(dc,1),t))
    if ang>30: obl+=1
    if dc<6: corner+=1
res.sort(reverse=True)
print('ends >30deg off normal:',obl,' ends within 6px of corner:',corner)
for r in res[:22]: print(r)
from collections import Counter
print(Counter(r[1] for r in res))
