import re,sys,glob,os,math
A,Bd=sys.argv[1],sys.argv[2]
def parse(p):
    s=open(p).read()
    masks={}
    for m in re.finditer(r'<mask id="([^"]+)"[^>]*>(.*?)</mask>',s,re.S):
        masks[m.group(1)]=[(float(a),float(b)) for a,b in re.findall(r'<circle cx="([-0-9.]+)" cy="([-0-9.]+)"',m.group(2))]
    edges=[]
    for m in re.finditer(r'<path d="([^"]+)"([^>]*)><title>([^<]*)</title>',s):
        d,attrs,title=m.groups()
        mk=re.search(r'mask="url\(#([^)]+)\)"',attrs)
        cuts=masks.get(mk.group(1),[]) if mk else []
        hops=[]
        toks=d.replace(',',' ').split()
        i=0;cur=None;arcs=[]
        while i<len(toks):
            t=toks[i]
            if t in 'ML': cur=(float(toks[i+1]),float(toks[i+2])); i+=3
            elif t=='C': cur=(float(toks[i+5]),float(toks[i+6])); i+=7
            elif t=='Q': cur=(float(toks[i+3]),float(toks[i+4])); i+=5
            elif t=='A':
                to=(float(toks[i+6]),float(toks[i+7])); arcs.append((cur,to,float(toks[i+1]))); cur=to; i+=8
            else: raise Exception(t)
        for k in range(0,len(arcs)-1,2):
            a,b=arcs[k],arcs[k+1]
            hops.append((min(a[0][0],b[1][0]),max(a[0][0],b[1][0]),max(a[0][1],b[1][1]),a[2]))
        edges.append((title,cuts,hops,d))
    return edges
tot=dict(files=0,edges=0,cuts_a=0,cuts_b=0,hops_a=0,hops_b=0,more=0,less=0,adj12_a=0,adj12_b=0,adj25_a=0,adj25_b=0,mixed_a=0,mixed_b=0,maxcut_a=0,maxcut_b=0)
worst=[]
def adj(c,h,D):
    n=0
    for (cx,cy) in c:
        for (x0,x1,y,r) in h:
            dx=max(x0-cx,cx-x1,0); dy=max(y-r-cy,cy-y,0)
            if math.hypot(dx,dy)<D: n+=1; break
    return n
for p in sorted(glob.glob(Bd+'/*.svg')):
    n=os.path.basename(p); q=A+'/'+n
    if not os.path.getsize(p) or not os.path.exists(q) or not os.path.getsize(q): continue
    try:
        a=parse(q); b=parse(p)
    except Exception: continue
    if len(a)!=len(b): continue
    tot['files']+=1
    for (t0,c0,h0,d0),(t1,c1,h1,d1) in zip(a,b):
        tot['edges']+=1; tot['cuts_a']+=len(c0); tot['cuts_b']+=len(c1); tot['hops_a']+=len(h0); tot['hops_b']+=len(h1)
        tot['maxcut_a']=max(tot['maxcut_a'],len(c0)); tot['maxcut_b']=max(tot['maxcut_b'],len(c1))
        if len(c1)>len(c0): tot['more']+=1; worst.append((len(c1)-len(c0),n,t1,len(c0),len(c1),len(h1)))
        if len(c1)<len(c0): tot['less']+=1
        if c0 and h0: tot['mixed_a']+=1
        if c1 and h1: tot['mixed_b']+=1
        tot['adj12_a']+=adj(c0,h0,12); tot['adj12_b']+=adj(c1,h1,12)
        tot['adj25_a']+=adj(c0,h0,25); tot['adj25_b']+=adj(c1,h1,25)
print(tot)
worst.sort(reverse=True)
for w in worst[:int(sys.argv[3]) if len(sys.argv)>3 else 8]: print(w)
