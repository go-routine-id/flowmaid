import re,sys
def pts(d):
    t=re.findall(r'[MLC]|-?\d+\.?\d*',d); i=0; out=[]; cur=None
    while i<len(t):
        c=t[i]; i+=1
        if c=='M' or c=='L':
            cur=(float(t[i]),float(t[i+1])); i+=2; out.append(cur)
        elif c=='C':
            p=[cur]+[(float(t[i+k]),float(t[i+k+1])) for k in (0,2,4)]; i+=6
            for s in range(1,17):
                u=s/16; a=(1-u)**3; b=3*(1-u)**2*u; cc=3*(1-u)*u*u; dd=u**3
                out.append((a*p[0][0]+b*p[1][0]+cc*p[2][0]+dd*p[3][0], a*p[0][1]+b*p[1][1]+cc*p[2][1]+dd*p[3][1]))
            cur=p[3]
    return out
def run(f,vertical=True):
    s=open(f).read()
    frames=[tuple(map(float,m.groups())) for m in re.finditer(r'<rect x="([\d.\-]+)" y="([\d.\-]+)" width="([\d.]+)" height="([\d.]+)" rx="8"',s)]
    lat=0; flowx=0; edges=0; latedges=0
    for m in re.finditer(r'<path d="(M [^"]*)" fill="none"',s):
        P=pts(m.group(1))
        if len(P)<2: continue
        edges+=1; hit=False
        for (x,y,w,h) in frames:
            ins=lambda p: x<p[0]<x+w and y<p[1]<y+h
            if ins(P[0])==ins(P[-1]): continue   # exactly one end inside
            for a,b in zip(P,P[1:]):
                if ins(a)!=ins(b):
                    # which wall: compare crossing point to walls
                    q=b if ins(a) else a
                    side = 'lat' if (q[0]<=x or q[0]>=x+w) else 'flow'
                    if not vertical: side = 'lat' if (q[1]<=y or q[1]>=y+h) else 'flow'
                    if side=='lat': lat+=1; hit=True
                    else: flowx+=1
        latedges+=hit
    print(f'{f}: frames {len(frames)}, edges {edges}, crossings through lateral walls {lat} ({latedges} edges), through flow-facing walls {flowx}')
for f in sys.argv[1:]: run(f)
