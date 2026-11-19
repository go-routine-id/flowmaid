import re,sys
def edges(p):
    s=open(p).read(); out=[]
    for m in re.finditer(r'<path d="([^"]+)"([^>]*)><title>([^<]*)</title>',s):
        d=m.group(1)
        # strip hops: "A r r 0 0 s x y [L x y] A r r 0 0 s x y" -> L to foot
        toks=d.replace(',',' ').split(); pts=[]; i=0
        while i<len(toks):
            t=toks[i]
            if t in 'ML': pts.append((float(toks[i+1]),float(toks[i+2]))); i+=3
            elif t=='C': pts.append((float(toks[i+5]),float(toks[i+6]))); i+=7
            elif t=='Q': pts.append((float(toks[i+3]),float(toks[i+4]))); i+=5
            elif t=='A': i+=8
            else: raise Exception(t)
        out.append((m.group(3),pts,d))
    return out
a=edges(sys.argv[1]); b=edges(sys.argv[2])
assert len(a)==len(b)
def bb(p): return (min(x for x,y in p),min(y for x,y in p),max(x for x,y in p),max(y for x,y in p))
n=0
for (t,pa,da),(t2,pb,db) in zip(a,b):
    # compare corner sets ignoring points on straight lines (hop feet add L points)
    def key(p):
        out=[]
        for q in p:
            if len(out)>=2:
                (x0,y0),(x1,y1)=out[-2],out[-1]
                if abs((x1-x0)*(q[1]-y0)-(y1-y0)*(q[0]-x0))<0.2 and (q[0]-x1)*(x1-x0)+(q[1]-y1)*(y1-y0)>=0: out[-1]=q; continue
            out.append(q)
        return out
    ka,kb=key(pa),key(pb)
    if len(ka)!=len(kb) or any(abs(x-u)>0.3 or abs(y-v)>0.3 for (x,y),(u,v) in zip(ka,kb)):
        n+=1; print('CHANGED',t,'bbox',tuple(round(v) for v in bb(pb)))
print(n,'of',len(a),'edges changed geometry')
