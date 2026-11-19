import re,sys,glob,os,math
V=sys.argv[1]
def parse(p):
    s=open(p).read()
    masks={}
    for m in re.finditer(r'<mask id="([^"]+)"[^>]*>(.*?)</mask>',s,re.S):
        masks[m.group(1)]=[(float(a),float(b)) for a,b in re.findall(r'<circle cx="([-0-9.]+)" cy="([-0-9.]+)"',m.group(2))]
        # other hole shapes
        masks[m.group(1)]+= [(None,None)]*len(re.findall(r'<(?:rect|path|line)[^>]*fill="#000"',m.group(2)))
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
tot=dict(files=0,edges=0,cuts_pre=0,cuts_now=0,hops=0,more=0,less=0,adj=0,mixed=0,maxcut_pre=0,maxcut_now=0)
worst=[]
for p in sorted(glob.glob(V+'/bulksvg/now/*.svg')):
    n=os.path.basename(p); q=V+'/bulksvg/pre/'+n
    if not os.path.getsize(p) or not os.path.exists(q) or not os.path.getsize(q): continue
    a=parse(q); b=parse(p)
    if len(a)!=len(b): continue
    tot['files']+=1
    for (t0,c0,h0,d0),(t1,c1,h1,d1) in zip(a,b):
        tot['edges']+=1; tot['cuts_pre']+=len(c0); tot['cuts_now']+=len(c1); tot['hops']+=len(h1)
        tot['maxcut_pre']=max(tot['maxcut_pre'],len(c0)); tot['maxcut_now']=max(tot['maxcut_now'],len(c1))
        if len(c1)>len(c0): tot['more']+=1; worst.append((len(c1)-len(c0),n,t1,len(c0),len(c1),len(h1)))
        if len(c1)<len(c0): tot['less']+=1
        if c1 and h1:
            tot['mixed']+=1
            for (cx,cy) in c1:
                if cx is None: continue
                for (x0,x1,y,r) in h1:
                    if abs(cy-y)<1.5 and (x0-12<=cx<=x1+12): tot['adj']+=1; break
print(tot)
worst.sort(reverse=True)
for w in worst[:25]: print(w)
