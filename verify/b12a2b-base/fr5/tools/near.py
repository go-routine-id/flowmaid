import re, sys, math
# near.py svg [thr]: pairs of edge paths whose sampled points come within thr px over a run (length of the run)
def parse(d):
    toks = re.findall(r'[MLCAZ]|-?\d+\.?\d*', d); pts=[]; i=0; cmd=None; cur=(0,0)
    while i < len(toks):
        t=toks[i]
        if t in 'MLCAZ': cmd=t; i+=1; continue
        if cmd in 'ML':
            cur=(float(toks[i]),float(toks[i+1])); pts.append(cur); i+=2
        elif cmd=='C':
            c=[float(x) for x in toks[i:i+6]]; p0=cur
            for k in range(1,9):
                s=k/8; u=1-s
                pts.append((u**3*p0[0]+3*u*u*s*c[0]+3*u*s*s*c[2]+s**3*c[4], u**3*p0[1]+3*u*u*s*c[1]+3*u*s*s*c[3]+s**3*c[5]))
            cur=(c[4],c[5]); i+=6
        elif cmd=='A':
            cur=(float(toks[i+5]),float(toks[i+6])); pts.append(cur); i+=7
        else: i+=1
    # densify
    out=[]
    for a,b in zip(pts,pts[1:]):
        L=math.dist(a,b); n=max(1,int(L/1.0))
        for k in range(n): out.append((a[0]+(b[0]-a[0])*k/n, a[1]+(b[1]-a[1])*k/n))
    out.append(pts[-1]); return out
s=open(sys.argv[1]).read(); thr=float(sys.argv[2]) if len(sys.argv)>2 else 2.0
E=[(m.group(2), parse(m.group(1))) for m in re.finditer(r'<path d="([^"]+)"[^>]*>\s*<title>([^<]+)</title>', s)]
for i in range(len(E)):
    for j in range(i+1,len(E)):
        a,b=E[i][1],E[j][1]; run=0
        for p in a[3:-3]:
            if min(math.dist(p,q) for q in b) < thr: run+=1
        if run>3: print(f"{run:4d} px  {E[i][0]}  ||  {E[j][0]}")
