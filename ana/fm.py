import re,sys,math
s=open(sys.argv[1]).read()
def sample(d,n=40):
    toks=re.findall(r'[MLCZ]|-?\d+\.?\d*',d)
    pts=[];i=0;cur=None;cmd=None
    while i<len(toks):
        t=toks[i]
        if t in 'MLCZ': cmd=t;i+=1;continue
        if cmd=='M': cur=(float(toks[i]),float(toks[i+1]));pts.append(cur);i+=2
        elif cmd=='L':
            nx=(float(toks[i]),float(toks[i+1]));
            for k in range(1,n+1):
                u=k/n;pts.append((cur[0]+(nx[0]-cur[0])*u,cur[1]+(nx[1]-cur[1])*u))
            cur=nx;i+=2
        elif cmd=='C':
            c1=(float(toks[i]),float(toks[i+1]));c2=(float(toks[i+2]),float(toks[i+3]));e=(float(toks[i+4]),float(toks[i+5]))
            for k in range(1,n+1):
                u=k/n;a=(1-u)**3;b=3*(1-u)**2*u;c=3*(1-u)*u*u;dd=u**3
                pts.append((a*cur[0]+b*c1[0]+c*c2[0]+dd*e[0],a*cur[1]+b*c1[1]+c*c2[1]+dd*e[1]))
            cur=e;i+=6
    return pts
paths=re.findall(r'<path d="([^"]*)"([^>]*)><title>([^<]*)</title>',s)
labels=re.findall(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="20" rx="4" fill="#ffffff" stroke="#d5d9ec"/>\s*<text[^>]*>([^<]*)</text>',s)
nodes=[]
for m in re.finditer(r'<g[^>]*>\s*(.*?)</g>',s,re.S):
    pass
# node rects
noderects=[tuple(map(float,m.groups())) for m in re.finditer(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)" rx="(?:3.0|19.0|36.0|27.5)"',s)]
print('labels',len(labels),'noderects',len(noderects))
P={t:sample(d) for d,a,t in paths}
for x,y,w,h_t in [(l[0],l[1],l[2],l[3]) for l in labels]:
    x,y,w=float(x),float(y),float(w);cx,cy=x+w/2,y+10
    txt=h_t
    own=[t for t in P if t.endswith(': '+txt)]
    best=min(own,key=lambda t:min(math.hypot(px-cx,py-cy) for px,py in P[t]))
    dist=min(math.hypot(px-cx,py-cy) for px,py in P[best])
    # other edges crossing label box
    cross=[t for t in P if t!=best and any(x<=px<=x+w and y<=py<=y+20 for px,py in P[t])]
    print(f'{txt!r:30} own={best.split(":")[0]} dist={dist:.1f} crossedBy={cross}')

print('---- edge through node check')
groups=re.findall(r'<g><title>([^<]*)</title>(.*?)</g>',s,re.S)
boxes=[]
for title,body in groups:
    m=re.search(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)"',body)
    if m:
        x,y,w,h=map(float,m.groups())
    else:
        nums=list(map(float,re.findall(r'-?\d+\.\d+',body.split('<text')[0])))
        m2=re.search(r'M ([\d.]+) ([\d.]+) A ([\d.]+) ([\d.]+) 0 0 0 ([\d.]+) [\d.]+ L [\d.]+ ([\d.]+)',body)
        x0,y0,rx,ry,x1,y1=map(float,m2.groups()); x,y,w,h=x0,y0-ry,x1-x0,y1-y0+2*ry
    boxes.append((title,x,y,w,h))
print(len(boxes))
for t,pts in P.items():
    for title,x,y,w,h in boxes:
        inside=[p for p in pts[3:-3] if x+3<p[0]<x+w-3 and y+3<p[1]<y+h-3]
        if inside: print(t,'passes through',title[:30],len(inside))

print('---- cluster title overlaps')
tit=[tuple(map(float,m.groups())) for m in re.finditer(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)" rx="4" fill="#f7f8fd"',s)]
for t,pts in P.items():
    for x,y,w,h in tit:
        if any(x<p[0]<x+w and y<p[1]<y+h for p in pts): print(t,'crosses cluster title at',x,y)
lb=[(float(l[0]),float(l[1]),float(l[2]),l[3]) for l in labels]
for x,y,w,txt in lb:
    for X,Y,W,H in tit:
        if x<X+W and X<x+w and y<Y+H and Y<y+20: print('label',txt,'overlaps title',X,Y)
    for title,X,Y,W,H in boxes:
        if x<X+W and X<x+w and y<Y+H and Y<y+20: print('label',txt,'overlaps node',title)
# crossings count
def segs(pts): return list(zip(pts,pts[1:]))
def inter(a,b,c,d):
    def cr(o,p,q): return (p[0]-o[0])*(q[1]-o[1])-(p[1]-o[1])*(q[0]-o[0])
    return cr(a,b,c)*cr(a,b,d)<0 and cr(c,d,a)*cr(c,d,b)<0
keys=list(P); n=0
for i in range(len(keys)):
    for j in range(i+1,len(keys)):
        A=P[keys[i]][::2];B=P[keys[j]][::2]
        c=0
        for a,b in segs(A):
            for cc,d in segs(B):
                if max(a[0],b[0])<min(cc[0],d[0]) or max(cc[0],d[0])<min(a[0],b[0]) or max(a[1],b[1])<min(cc[1],d[1]) or max(cc[1],d[1])<min(a[1],b[1]): continue
                if inter(a,b,cc,d): c+=1
        n+=c
print('total crossings approx',n)
