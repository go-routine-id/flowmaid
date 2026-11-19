import re,sys,html
s=open(sys.argv[1]).read()
edges=re.findall(r'<path d="(M [^"]*)"[^>]*><title>([^<]*)</title></path>',s)
texts=re.findall(r'<text x="([-0-9.]+)" y="([-0-9.]+)"[^>]*>([^<]*)</text>',s)
tpos={}
for x,y,t in texts:
    tpos.setdefault(html.unescape(t),[]).append((float(x),float(y)))
for d,title in edges:
    title=html.unescape(title)
    if ': ' not in title: continue
    name,lab=title.split(': ',1)
    lab=lab.strip()
    # collect L segments (straight legs)
    pts=[]
    cur=None
    legs=[]
    for c,args in re.findall(r'([MLC])([^MLC]*)',d):
        v=list(map(float,re.findall(r'-?[0-9.]+',args)))
        p=(v[-2],v[-1])
        if c=='L' and cur: legs.append((cur,p))
        cur=p
    # merge collinear legs
    m=[]
    for a,b in legs:
        if m and ((abs(m[-1][0][0]-b[0])<0.3 and abs(m[-1][1][0]-a[0])<0.3) or (abs(m[-1][0][1]-b[1])<0.3 and abs(m[-1][1][1]-a[1])<0.3)) and abs(m[-1][1][0]-a[0])<0.3 and abs(m[-1][1][1]-a[1])<0.3:
            m[-1]=(m[-1][0],b)
        else: m.append((a,b))
    lp=tpos.get(lab.split('\n')[0]) or [None]
    # pick label nearest to path
    best=None
    for q in lp:
        if q is None: continue
        dmin=1e9;idx=None
        for i,(a,b) in enumerate(m):
            if abs(a[0]-b[0])<0.3:
                dd=abs(q[0]-a[0]) + max(0,min(a[1],b[1])-q[1],q[1]-max(a[1],b[1]))
            elif abs(a[1]-b[1])<0.3:
                dd=abs(q[1]-a[1]) + max(0,min(a[0],b[0])-q[0],q[0]-max(a[0],b[0]))
            else: dd=1e9
            if dd<dmin: dmin=dd;idx=i
        if best is None or dmin<best[0]: best=(dmin,idx,q)
    if best is None: print(name,lab,'NOLABEL'); continue
    n=len(m)
    kind='none' if best[1] is None else ('first' if best[1]==0 else ('last' if best[1]==n-1 else 'mid%d'%best[1]))
    print(f"{name:20s} {lab[:20]:20s} legs={n} label@{best[2]} on={kind} dist={best[0]:.1f}")
