# Count drawn crossings between edge paths of an SVG (mermaid or flowmaid).
import re,sys,math
NUM=re.compile(r'-?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?')
def sample(d):
    toks=re.findall(r'[A-Za-z]|-?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?',d)
    i=0;cur=(0.0,0.0);pts=[];cmd=None
    while i<len(toks):
        if toks[i].isalpha():
            cmd=toks[i]; i+=1; continue
        need={'M':2,'L':2,'Q':4,'C':6,'A':7,'H':1,'V':1}.get(cmd.upper(),0)
        if need==0 or i+need>len(toks): break
        v=[float(x) for x in toks[i:i+need]]; i+=need
        if cmd=='M': cur=(v[0],v[1]); pts.append(cur); cmd='L'
        elif cmd=='L': cur=(v[0],v[1]); pts.append(cur)
        elif cmd=='A': cur=(v[5],v[6]); pts.append(cur)
        elif cmd=='H': cur=(v[0],cur[1]); pts.append(cur)
        elif cmd=='V': cur=(cur[0],v[0]); pts.append(cur)
        elif cmd=='Q':
            c=(v[0],v[1]);p=(v[2],v[3])
            for k in range(1,9):
                s=k/8;u=1-s
                pts.append((u*u*cur[0]+2*u*s*c[0]+s*s*p[0],u*u*cur[1]+2*u*s*c[1]+s*s*p[1]))
            cur=p
        elif cmd=='C':
            c1=(v[0],v[1]);c2=(v[2],v[3]);p=(v[4],v[5])
            for k in range(1,13):
                s=k/12;u=1-s
                pts.append((u**3*cur[0]+3*u*u*s*c1[0]+3*u*s*s*c2[0]+s**3*p[0],u**3*cur[1]+3*u*u*s*c1[1]+3*u*s*s*c2[1]+s**3*p[1]))
            cur=p
    return pts
def inter(a,b,c,d):
    def cr(o,p,q): return (p[0]-o[0])*(q[1]-o[1])-(p[1]-o[1])*(q[0]-o[0])
    d1=cr(c,d,a);d2=cr(c,d,b);d3=cr(a,b,c);d4=cr(a,b,d)
    if d1*d2<0 and d3*d4<0:
        t=d1/(d1-d2); return (a[0]+t*(b[0]-a[0]),a[1]+t*(b[1]-a[1]))
    return None
svg=open(sys.argv[1]).read()
kind=sys.argv[2] if len(sys.argv)>2 else 'mermaid'
if kind=='mermaid':
    ds=re.findall(r'<path d="([^"]+)"[^>]*flowchart-link',svg)
else:
    ds=re.findall(r'<path d="(M [^"]+)"[^>]*stroke-width',svg)
polys=[sample(d) for d in ds]
n=0
for i in range(len(polys)):
    for j in range(i+1,len(polys)):
        A,B=polys[i],polys[j]
        hits=[]
        for k in range(len(A)-1):
            for l in range(len(B)-1):
                p=inter(A[k],A[k+1],B[l],B[l+1])
                if p and all(math.dist(p,q)>4 for q in hits):
                    if min(math.dist(p,A[0]),math.dist(p,A[-1]),math.dist(p,B[0]),math.dist(p,B[-1]))>6:
                        hits.append(p)
        n+=len(hits)
print(len(polys),'edges',n,'crossings')
