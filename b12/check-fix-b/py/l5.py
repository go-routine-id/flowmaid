# L5 of SPEC 2.3 as written: longest path, then (1) source blocks sink, (2) no interleaving, repeated.
import random, sys
def solve(n, rows, cons, lb):
    # longest path over constraint DAG cons: (u,v) => start[v] >= start[u]+rows[u]; lb lower bounds
    indeg=[0]*n; adj=[[] for _ in range(n)]
    for u,v in cons: adj[u].append(v); indeg[v]+=1
    start=list(lb); q=[i for i in range(n) if indeg[i]==0]; seen=0
    while q:
        u=q.pop(0); seen+=1
        for v in adj[u]:
            start[v]=max(start[v], start[u]+rows[u]); indeg[v]-=1
            if indeg[v]==0: q.append(v)
    return start if seen==n else None
def l5(n, rows, isblock, links, variant, cap=2000):
    cons=set(links); lb=[0]*n
    start=solve(n,rows,cons,lb)
    it=0
    while True:
        it+=1
        if it>cap: return ('diverge', start)
        changed=False
        # rule 1
        for s in range(n):
            if not isblock[s]: continue
            into = any(v==s for (u,v) in (links if variant=='orig' else cons))
            if into: continue
            succ=[v for (u,v) in links if u==s]
            if not succ: continue
            t=min(start[v] for v in succ)-rows[s]
            if t>start[s]:
                lb[s]=t; start=solve(n,rows,cons,lb); changed=True
                if start is None: return ('cycle', None)
        # rule 2
        while True:
            pair=None
            for a in range(n):
                for b in range(n):
                    if a!=b and isblock[a] and isblock[b]:
                        ea=start[a]+rows[a]; eb=start[b]+rows[b]
                        if start[a]<start[b]<ea<eb: pair=(a,b); break
                if pair: break
            if not pair: break
            cons.add(pair); start=solve(n,rows,cons,lb); changed=True
            if start is None: return ('cycle', None)
            if max(start)>10*sum(rows): return ('diverge', start)
        if not changed: break
    # final check
    for a in range(n):
        for b in range(n):
            if a!=b and isblock[a] and isblock[b]:
                if start[a]<start[b]<start[a]+rows[a]<start[b]+rows[b]: return ('interleaved', start)
    return ('ok', start, it)
random.seed(int(sys.argv[1]) if len(sys.argv)>1 else 1)
res={}
maxit=0
for trial in range(60000):
    n=random.randint(3,9)
    isblock=[random.random()<0.6 for _ in range(n)]
    rows=[random.randint(1,4) if isblock[i] else 1 for i in range(n)]
    order=list(range(n)); random.shuffle(order)
    links=set()
    for _ in range(random.randint(1,2*n)):
        i,j=sorted(random.sample(range(n),2))
        links.add((order[i],order[j]))
    for variant in ('orig','all'):
        r=l5(n,rows,isblock,sorted(links),variant)
        res[(variant,r[0])]=res.get((variant,r[0]),0)+1
        if r[0]=='ok': maxit=max(maxit,r[2])
        if r[0]!='ok' and res[(variant,r[0])]<=2:
            print(variant, r[0], 'rows',rows,'block',isblock,'links',sorted(links), r[1])
print(res,'max rounds',maxit)
