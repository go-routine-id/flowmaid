import re,sys,math
E=[]
for l in open(sys.argv[1]):
    m=re.match(r'edge (\S+)->(\S+) bez \(([-\d.]+),([-\d.]+)\) \(([-\d.]+),([-\d.]+)\) \(([-\d.]+),([-\d.]+)\) \(([-\d.]+),([-\d.]+)\) wps(.*)',l)
    if not m: continue
    w=m.group(11).split()
    pts=[tuple(map(float,p.strip('()').split(','))) for p in w] if w else [(float(m.group(3)),float(m.group(4))),(float(m.group(9)),float(m.group(10)))]
    E.append((m.group(1),m.group(2),pts))
def pd(p,a,b):
    dx,dy=b[0]-a[0],b[1]-a[1]; l=dx*dx+dy*dy
    t=0 if l==0 else max(0,min(1,((p[0]-a[0])*dx+(p[1]-a[1])*dy)/l))
    return math.hypot(p[0]-a[0]-t*dx,p[1]-a[1]-t*dy)
def cross(a,b,c,d):
    s=lambda p,q,r:(q[0]-p[0])*(r[1]-p[1])-(q[1]-p[1])*(r[0]-p[0])
    return s(a,b,c)*s(a,b,d)<0 and s(c,d,a)*s(c,d,b)<0
res=[]
for i in range(len(E)):
    for j in range(i+1,len(E)):
        a,b=E[i],E[j]
        if {a[0],a[1]}&{b[0],b[1]}: continue
        sa=list(zip(a[2],a[2][1:])); sb=list(zip(b[2],b[2][1:]))
        if any(cross(p,q,r,s) for p,q in sa for r,s in sb): continue
        d=min(min(pd(p,r,s),pd(q,r,s),pd(r,p,q),pd(s,p,q)) for p,q in sa for r,s in sb)
        res.append((round(d,2),a[0]+'->'+a[1],b[0]+'->'+b[1]))
res.sort(); print(res[:5])
