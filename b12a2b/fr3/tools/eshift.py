import re,sys
def parse(p):
    s=open(p).read(); out={}
    for m in re.finditer(r'<path([^>]*)>\s*<title>([^<]*)</title>',s):
        d=re.search(r' d="([^"]*)"',m.group(1))
        if d: out.setdefault(m.group(2),[]).append([float(x) for x in re.findall(r'-?[\d.]+',d.group(1))])
    return out
A,B=sys.argv[1],sys.argv[2]
for f in sys.argv[3:]:
    a=parse(A+'/'+f); b=parse(B+'/'+f)
    for k in b:
        for j,pbk in enumerate(b[k]):
            pak=a.get(k,[None]*(j+1))[j] if j < len(a.get(k,[])) else None
            if pak!=pbk:
                pa=list(zip(pak[::2],pak[1::2])) if pak else [(0,0)]; pb=list(zip(pbk[::2],pbk[1::2]))
                def dmax(P,Q): return max(min(((p[0]-q[0])**2+(p[1]-q[1])**2)**.5 for q in Q) for p in P)
                print(f"{f}\t{k}\t{max(dmax(pa,pb),dmax(pb,pa)):.1f} px\tnpts {len(pa)}->{len(pb)}")
