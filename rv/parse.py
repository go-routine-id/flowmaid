import re, sys, math
def load(svgpath, mmdpath):
    s=open(svgpath).read()
    m=open(mmdpath).read()
    labels={}
    for mm in re.finditer(r'(n\d+)(?:\[\(|\(\[|\[)"([^"]*)"', m):
        labels[mm.group(1)]=mm.group(2).replace('<br/>',' ')
    lab2id={v:k for k,v in labels.items()}
    nodes={}
    for g in re.finditer(r'<g><title>([^<]*)</title>\n(.*?)</g>', s, re.S):
        t=g.group(1); body=g.group(2)
        nid=lab2id.get(t)
        r=re.search(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)"', body)
        if r:
            x,y,w,h=map(float,r.groups())
        else:
            p=re.search(r'<path d="M ([\d.]+) ([\d.]+) A ([\d.]+) ([\d.]+) 0 0 0 ([\d.]+) [\d.]+ L [\d.]+ ([\d.]+)', body)
            x1,y1,rx,ry,x2,y2=map(float,p.groups())
            x,y,w,h=x1,y1-ry,x2-x1,y2-y1+2*ry
        nodes[nid]=(x,y,w,h,t)
    clusters=[tuple(map(float,c)) for c in re.findall(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)" rx="8" fill="#f7f8fd"',s)]
    edges=[]
    for p in re.finditer(r'<path d="([^"]*)"([^>]*)><title>([^<]*)</title>', s):
        d,attrs,t=p.groups()
        edges.append((t,d,attrs))
    elabels=[tuple(map(float,c)) for c in re.findall(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="20" rx="4" fill="#ffffff"',s)]
    return nodes, clusters, edges, elabels, s

def sample(d, n=20):
    toks=re.findall(r'[MLC]|-?[\d.]+', d)
    pts=[]; i=0; cur=None; cmd=None
    out=[]
    while i<len(toks):
        if toks[i] in 'MLC': cmd=toks[i]; i+=1; continue
        if cmd=='M':
            cur=(float(toks[i]),float(toks[i+1])); i+=2; out.append(cur)
        elif cmd=='L':
            p=(float(toks[i]),float(toks[i+1])); i+=2
            for k in range(1,n+1):
                t=k/n; out.append((cur[0]+(p[0]-cur[0])*t, cur[1]+(p[1]-cur[1])*t))
            cur=p
        elif cmd=='C':
            c1=(float(toks[i]),float(toks[i+1])); c2=(float(toks[i+2]),float(toks[i+3])); p=(float(toks[i+4]),float(toks[i+5])); i+=6
            for k in range(1,n+1):
                t=k/n; mt=1-t
                out.append((mt**3*cur[0]+3*mt*mt*t*c1[0]+3*mt*t*t*c2[0]+t**3*p[0], mt**3*cur[1]+3*mt*mt*t*c1[1]+3*mt*t*t*c2[1]+t**3*p[1]))
            cur=p
    return out
