# Independent re-implementation of SPEC 2.3 L5 with several readings.
import random, sys, itertools
def solve(n, rows, cons, lb):
    indeg=[0]*n; adj=[[] for _ in range(n)]
    for u,v in cons: adj[u].append(v); indeg[v]+=1
    start=list(lb); q=[i for i in range(n) if indeg[i]==0]; seen=0
    while q:
        u=q.pop(0); seen+=1
        for v in adj[u]:
            start[v]=max(start[v], start[u]+rows[u]); indeg[v]-=1
            if indeg[v]==0: q.append(v)
    return start if seen==n else None

def interleaved_pairs(n, rows, isblock, start):
    out=[]
    for a in range(n):
        for b in range(n):
            if a!=b and isblock[a] and isblock[b]:
                ea=start[a]+rows[a]; eb=start[b]+rows[b]
                if start[a]<start[b]<ea<eb: out.append((a,b))
    return out

def l5(n, rows, isblock, links, succ_mode, into_mode, pair_mode, cap=400):
    """succ_mode: 'links' (units it links to) | 'cons' (every constraint out of it)
       into_mode: 'links' | 'cons'
       pair_mode: 'key' (first pair in key order: a then b) | 'start' (earliest start(A), start(B), key)"""
    cons=set(links); lb=[0]*n
    start=solve(n,rows,cons,lb)
    if start is None: return ('cycle0',None,0,0)
    rounds=0; added=0
    while True:
        rounds+=1
        if rounds>cap: return ('diverge', start, rounds, added)
        changed=False
        for s in range(n):
            if not isblock[s]: continue
            src = links if into_mode=='links' else cons
            if any(v==s for (u,v) in src): continue
            src2 = links if succ_mode=='links' else cons
            succ=[v for (u,v) in src2 if u==s]
            if not succ: continue
            t=min(start[v] for v in succ)-rows[s]
            if t>start[s]:
                lb[s]=t; new=solve(n,rows,cons,lb); changed=True
                if new is None: return ('cycle', None, rounds, added)
                start=new
        while True:
            ps=interleaved_pairs(n,rows,isblock,start)
            if not ps: break
            if pair_mode=='key': pair=min(ps)
            else: pair=min(ps, key=lambda p:(start[p[0]],start[p[1]],p[0],p[1]))
            if pair in cons: return ('dup', start, rounds, added)
            cons.add(pair); added+=1
            new=solve(n,rows,cons,lb); changed=True
            if new is None: return ('cycle', None, rounds, added)
            start=new
            if max(start)>50*sum(rows): return ('diverge', start, rounds, added)
        if not changed: break
    if interleaved_pairs(n,rows,isblock,start): return ('interleaved', start, rounds, added)
    return ('ok', start, rounds, added)

if __name__=='__main__':
    # l5loop: units in key order: B(0,3 rows) T(1,4 rows) S(2,2 rows) n1(3) n4(4)
    rows=[3,4,2,1,1]; isb=[True,True,True,False,False]
    links=[(3,1),(4,0),(2,1)]
    for sm in ('links','cons'):
        for im in ('links','cons'):
            for pm in ('key','start'):
                r=l5(5,rows,isb,links,sm,im,pm,cap=50)
                print('l5loop', sm, im, pm, r)
    # interleave.mmd (critic): N0->T(3 rows); N0->P1..P5->Q; S(2 rows)->Q
    # units: declare order unknown; use N0,T,P1..P5,Q,S
    seed=int(sys.argv[1]) if len(sys.argv)>1 else 1
    N=int(sys.argv[2]) if len(sys.argv)>2 else 40000
    random.seed(seed)
    res={}; maxr={}; worst={}
    for trial in range(N):
        n=random.randint(3,9)
        isblock=[random.random()<0.6 for _ in range(n)]
        rows=[random.randint(1,4) if isblock[i] else 1 for i in range(n)]
        order=list(range(n)); random.shuffle(order)
        links=set()
        for _ in range(random.randint(1,2*n)):
            i,j=sorted(random.sample(range(n),2))
            links.add((order[i],order[j]))
        links=sorted(links)
        nb=sum(isblock); pairs=nb*(nb-1)//2
        for sm,im,pm in (('links','links','key'),('links','cons','key'),('cons','cons','key'),('cons','cons','start'),('links','cons','start')):
            r=l5(n,rows,isblock,links,sm,im,pm)
            k=(sm,im,pm)
            res[(k,r[0])]=res.get((k,r[0]),0)+1
            if r[0]=='ok':
                maxr[k]=max(maxr.get(k,0), r[2])
                # rounds vs pairs+1
                if r[2]>pairs+1:
                    worst[k]=worst.get(k,0)+1
                    if worst[k]<=2: print('rounds>pairs+1', k, 'rounds', r[2], 'pairs', pairs, rows, isblock, links)
    for k in sorted(res): print(k, res[k])
    print('max rounds', maxr); print('rounds>pairs+1 count', worst)
