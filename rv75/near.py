import re,sys,math
sys.path.insert(0,'/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rv75')
svg=open(sys.argv[1]).read()
svg=re.sub(r'<mask .*?</mask>','',svg,flags=re.S)
def sample(d,step=2.0):
    toks=re.findall(r'[MLC]|-?[\d.]+',d)
    pts=[];i=0;cur=None;cmd=None
    while i<len(toks):
        t=toks[i]
        if t in 'MLC': cmd=t;i+=1;continue
        if cmd=='M': cur=(float(toks[i]),float(toks[i+1]));pts.append(cur);i+=2
        elif cmd=='L':
            p=(float(toks[i]),float(toks[i+1]));n=max(1,int(math.dist(cur,p)/step))
            for k in range(1,n+1): pts.append((cur[0]+(p[0]-cur[0])*k/n,cur[1]+(p[1]-cur[1])*k/n))
            cur=p;i+=2
        elif cmd=='C':
            c1=(float(toks[i]),float(toks[i+1]));c2=(float(toks[i+2]),float(toks[i+3]));p=(float(toks[i+4]),float(toks[i+5]));i+=6
            L=math.dist(cur,c1)+math.dist(c1,c2)+math.dist(c2,p);n=max(2,int(L/step))
            for k in range(1,n+1):
                s=k/n;u=1-s
                pts.append((u**3*cur[0]+3*u*u*s*c1[0]+3*u*s*s*c2[0]+s**3*p[0],u**3*cur[1]+3*u*u*s*c1[1]+3*u*s*s*c2[1]+s**3*p[1]))
            cur=p
    return pts
E={}
for m in re.finditer(r'<path d="([^"]*)"[^>]*><title>([^<]*)</title>',svg):
    E[m.group(2)]=sample(m.group(1))
names=list(E)
# grid index
from collections import defaultdict
G=defaultdict(list)
for k,n in enumerate(names):
    for j,p in enumerate(E[n]): G[(int(p[0]//8),int(p[1]//8))].append((k,j))
pairs=defaultdict(int)
for k,n in enumerate(names):
    pts=E[n]
    for j,p in enumerate(pts):
        # skip near endpoints (port zone 20px)
        if j*2<20 or (len(pts)-j)*2<20: continue
        cx,cy=int(p[0]//8),int(p[1]//8); seen=set()
        for dx in (-1,0,1):
            for dy in (-1,0,1):
                for (k2,j2) in G[(cx+dx,cy+dy)]:
                    if k2<=k or k2 in seen: continue
                    q=E[names[k2]][j2]
                    if j2*2<20 or (len(E[names[k2]])-j2)*2<20: continue
                    if math.dist(p,q)<3.0: seen.add(k2)
        for k2 in seen: pairs[(n,names[k2])]+=2
top=sorted(pairs.items(),key=lambda x:-x[1])
print('pairs with >=20px run within 3px (excluding 20px port zones):',sum(1 for _,v in top if v>=20))
for p,v in top[:15]: print(v,p)
