import re,json
s=open('/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/cmp/out/bench-75.flowmaid.svg').read()
nodes=json.load(open('/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/cmp/crops75/nodes.json'))
def bez(p0,p1,p2,p3,n=40):
    for i in range(n+1):
        t=i/n; u=1-t
        yield (u**3*p0[0]+3*u*u*t*p1[0]+3*u*t*t*p2[0]+t**3*p3[0], u**3*p0[1]+3*u*u*t*p1[1]+3*u*t*t*p2[1]+t**3*p3[1])
def sample(d):
    toks=re.findall(r'[MLC]|-?[\d.]+',d)
    i=0;cur=None;pts=[]
    while i<len(toks):
        c=toks[i];i+=1
        if c=='M': cur=(float(toks[i]),float(toks[i+1]));i+=2;pts.append(cur)
        elif c=='L':
            nxt=(float(toks[i]),float(toks[i+1]));i+=2
            for k in range(1,21): pts.append((cur[0]+(nxt[0]-cur[0])*k/20,cur[1]+(nxt[1]-cur[1])*k/20))
            cur=nxt
        elif c=='C':
            p=[(float(toks[i+2*j]),float(toks[i+2*j+1])) for j in range(3)];i+=6
            pts+=list(bez(cur,p[0],p[1],p[2]))[1:];cur=p[2]
    return pts
edges={}
for m in re.finditer(r'<path d="([^"]+)"([^>]*)><title>([^<]+)</title>',s):
    d,attrs,t=m.groups(); edges[t]=sample(d)
