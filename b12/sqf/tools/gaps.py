import re,sys
# parse dump: edge lines with wps; compute closest parallel overlapping segments between different edges
segs=[]
for l in open(sys.argv[1]):
    m=re.match(r'edge (\S+) bez .* wps (.*)',l)
    if not m: continue
    pts=[tuple(map(float,p.strip('()').split(','))) for p in m.group(2).split()]
    for a,b in zip(pts,pts[1:]):
        if abs(a[0]-b[0])<1e-6 and abs(a[1]-b[1])>20: segs.append((m.group(1),'v',a[0],min(a[1],b[1]),max(a[1],b[1])))
        elif abs(a[1]-b[1])<1e-6 and abs(a[0]-b[0])>20: segs.append((m.group(1),'h',a[1],min(a[0],b[0]),max(a[0],b[0])))
res=[]
for i in range(len(segs)):
    for j in range(i+1,len(segs)):
        p,q=segs[i],segs[j]
        if p[0]==q[0] or p[1]!=q[1]: continue
        ov=min(p[4],q[4])-max(p[3],q[3])
        if ov<20: continue
        res.append((abs(p[2]-q[2]),p[0],q[0],p[1],p[2],q[2],round(ov)))
res.sort()
for r in res[:int(sys.argv[2]) if len(sys.argv)>2 else 8]: print(r)
