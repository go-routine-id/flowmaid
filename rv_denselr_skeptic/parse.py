import re,sys,math
def load(fn):
    s=open(fn).read()
    edges={}
    for m in re.finditer(r'<path d="([^"]*)"([^>]*)><title>([^<]*)</title>', s):
        d,attrs,t=m.groups()
        edges[t]=sample(d)
    labels=[]
    for m in re.finditer(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="20" rx="4" fill="#ffffff" stroke="#d5d9ec"/>\s*<text[^>]*>([^<]*)</text>', s):
        x,y,w,txt=m.groups(); labels.append((txt,float(x),float(y),float(w),20.0))
    nodes={}
    for m in re.finditer(r'<g><title>([^<]*)</title>\s*(<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)"|<path d="M ([\d.]+) ([\d.]+) A ([\d.]+) ([\d.]+) 0 0 0 ([\d.]+) [\d.]+ L [\d.]+ ([\d.]+))', s):
        t=m.group(1)
        if m.group(3):
            nodes[t]=tuple(float(m.group(i)) for i in (3,4,5,6))
        else:
            x0,y0,rx,ry,x1,y1=(float(m.group(i)) for i in (7,8,9,10,11,12))
            nodes[t]=(x0,y0-ry,x1-x0,y1-y0+2*ry)
    return s,edges,labels,nodes
def sample(d,n=40):
    toks=re.findall(r'[MLCQZ]|-?\d+\.?\d*',d)
    pts=[];cur=None;i=0;cmd=None
    out=[]
    while i<len(toks):
        if toks[i] in 'MLCQZ': cmd=toks[i]; i+=1; continue
        if cmd=='M':
            cur=(float(toks[i]),float(toks[i+1])); i+=2; out.append(cur)
        elif cmd=='L':
            p=(float(toks[i]),float(toks[i+1])); i+=2
            for k in range(1,n+1):
                t=k/n; out.append((cur[0]+(p[0]-cur[0])*t,cur[1]+(p[1]-cur[1])*t))
            cur=p
        elif cmd=='C':
            c1=(float(toks[i]),float(toks[i+1]));c2=(float(toks[i+2]),float(toks[i+3]));p=(float(toks[i+4]),float(toks[i+5])); i+=6
            for k in range(1,n+1):
                t=k/n;u=1-t
                out.append((u**3*cur[0]+3*u*u*t*c1[0]+3*u*t*t*c2[0]+t**3*p[0], u**3*cur[1]+3*u*u*t*c1[1]+3*u*t*t*c2[1]+t**3*p[1]))
            cur=p
        else: i+=1
    return out
