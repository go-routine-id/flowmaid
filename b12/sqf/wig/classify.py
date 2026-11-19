import re,sys,collections
D='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/sqf/wig/'
dirs={'bench-75':'LR','dense-lr':'LR','dense-td':'TD','wide-labels':None}
for l in open(D+'wiggles.txt'):
    f,mode,rest=l.strip().split('\t'); e,_,amt=rest.split(' ')
    u,v=e.split('->')
    frames={};nodes={}
    for d in open(D+f+'.dump'):
        m=re.match(r'frame (\S+) x ([-\d.]+)\.\.([-\d.]+) y ([-\d.]+)\.\.([-\d.]+)',d)
        if m: frames[m[1]]=tuple(map(float,m.groups()[1:]))
        m=re.match(r'node (\S+) c \(([-\d.]+),([-\d.]+)\) x ([-\d.]+)\.\.([-\d.]+) y ([-\d.]+)\.\.([-\d.]+)',d)
        if m: nodes[m[1]]=tuple(map(float,m.groups()[3:]))
    src=open(D+f+'.mmd').read()
    hd=re.search(r'^\s*(flowchart|graph)\s+(\w+)',src,re.M).group(2)
    vert = hd in ('TD','TB','BT')
    def inside(n,F): return n[0]>=F[0]-1e-6 and n[1]<=F[1]+1e-6 and n[2]>=F[2]-1e-6 and n[3]<=F[3]+1e-6
    if u not in nodes or v not in nodes: print(f,e,amt,'(frame end)'); continue
    A,B=nodes[u],nodes[v]
    # lateral = x for vertical flow, y for horizontal; along = other
    lat=lambda r:(r[0],r[1]) if vert else (r[2],r[3])
    alo=lambda r:(r[2],r[3]) if vert else (r[0],r[1])
    la,lb=lat(A),lat(B); hull=(min(la[0],lb[0]),max(la[1],lb[1]))
    a0=min(alo(A)[1],alo(B)[1]); a1=max(alo(A)[0],alo(B)[0])  # between the two boxes along flow
    if alo(A)[0] > alo(B)[0]: back=True
    else: back=False
    foreign=[(k,F) for k,F in frames.items() if not inside(A,F) and not inside(B,F)]
    blockers=[(k,F) for k,F in foreign if alo(F)[1]>a0 and alo(F)[0]<a1]
    # free column in hull through all blockers
    iv=[lat(F) for k,F in blockers]
    iv.sort(); free=[]; cur=hull[0]
    for lo,hi in iv:
        if lo>cur: free.append((cur,min(lo,hull[1])))
        cur=max(cur,hi)
    if cur<hull[1]: free.append((cur,hull[1]))
    free=[x for x in free if x[1]-x[0]>8]
    cls='free-column' if free else 'blocked'
    print(f"{f:12} {e:16} {float(amt):8.1f} {'back' if back else 'fwd '} {cls:12} blockers={[k for k,_ in blockers]} free={[(round(a),round(b)) for a,b in free][:3]}")
