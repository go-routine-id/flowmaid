import re,sys
def parse(p):
    s=open(p).read(); out={}
    for m in re.finditer(r'<path([^>]*)>\s*<title>([^<]*)</title>',s):
        d=re.search(r' d="([^"]*)"',m.group(1))
        if d: out[m.group(2)]=[float(x) for x in re.findall(r'-?[\d.]+',d.group(1))]
    return out
for f in sys.argv[2:]:
    a=parse(sys.argv[1]+'/'+f); b=parse('new/'+f)
    for k in b:
        if a.get(k)!=b[k]:
            pa=list(zip(a[k][::2],a[k][1::2])); pb=list(zip(b[k][::2],b[k][1::2]))
            # hausdorff-ish on control points
            def dmax(P,Q): return max(min(((p[0]-q[0])**2+(p[1]-q[1])**2)**.5 for q in Q) for p in P)
            print(f"{f}\t{k}\t{max(dmax(pa,pb),dmax(pb,pa)):.1f} px\tnpts {len(pa)}->{len(pb)}")
