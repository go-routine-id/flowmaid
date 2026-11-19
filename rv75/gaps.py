import re,sys,math
svg=open(sys.argv[1]).read()
masks={}
for m in re.finditer(r'<mask id="([^"]+)"[^>]*>(.*?)</mask>',svg,flags=re.S):
    cs=[(float(a),float(b),float(r)) for a,b,r in re.findall(r'<circle cx="([\d.]+)" cy="([\d.]+)" r="([\d.]+)"',m.group(2))]
    masks[m.group(1)]=cs
print('masks',len(masks),'circles',sum(len(v) for v in masks.values()), 'radii',set(c[2] for v in masks.values() for c in v))
def sample(d):
    toks=re.findall(r'[MLC]|-?[\d.]+',d)
    pts=[];i=0;cur=None;cmd=None
    while i<len(toks):
        t=toks[i]
        if t in 'MLC': cmd=t;i+=1;continue
        if cmd=='M': cur=(float(toks[i]),float(toks[i+1]));pts.append(cur);i+=2
        elif cmd=='L': p=(float(toks[i]),float(toks[i+1]));pts.append(p);cur=p;i+=2
        elif cmd=='C':
            c1=(float(toks[i]),float(toks[i+1]));c2=(float(toks[i+2]),float(toks[i+3]));p=(float(toks[i+4]),float(toks[i+5]));i+=6
            for k in range(1,21):
                s=k/20;u=1-s
                pts.append((u**3*cur[0]+3*u*u*s*c1[0]+3*u*s*s*c2[0]+s**3*p[0],u**3*cur[1]+3*u*u*s*c1[1]+3*u*s*s*c2[1]+s**3*p[1]))
            cur=p
    return pts
close=0;masked=0;res=[]
for m in re.finditer(r'<path d="([^"]*)"[^>]*?mask="url\(#([^)]*)\)"[^>]*><title>([^<]*)</title>',svg):
    d,mid,t=m.groups(); pts=sample(d)
    cum=[0]
    for a,b in zip(pts,pts[1:]): cum.append(cum[-1]+math.dist(a,b))
    pos=[]
    for cx,cy,r in masks[mid]:
        j=min(range(len(pts)),key=lambda k:math.dist(pts[k],(cx,cy)))
        pos.append(cum[j])
    pos.sort(); sp=[round(b-a) for a,b in zip(pos,pos[1:])]
    masked+=1
    if any(s<40 for s in sp): close+=1
    res.append((t,len(pos),round(cum[-1]),sp))
print('masked edges',masked,'with two gaps <40px apart',close)
for r in sorted(res,key=lambda r:-r[1])[:12]: print(r)
