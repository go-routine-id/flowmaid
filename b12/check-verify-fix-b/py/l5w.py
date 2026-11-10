import random, sys
sys.path.insert(0, '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/check-verify-fix-b/py')
from l5v import l5
# interleave.mmd, units by key (first mention). Filled in below from the file.
def run(name, units, rows, isb, links):
    idx={u:i for i,u in enumerate(units)}
    L=sorted((idx[a],idx[b]) for a,b in links)
    for sm,im,pm in (('cons','cons','key'),('cons','cons','start'),('cons','links','start')):
        r=l5(len(units),rows,isb,L,sm,im,pm)
        print(name, sm,im,pm, r[0], {u:(r[1][i], r[1][i]+rows[i]-1) for i,u in enumerate(units)} if r[1] else None, 'rounds',r[2],'added',r[3])
# interleave: keys: N0, S(s1), T(t1), P1..P5, Q
units=['N0','S','T','P1','P2','P3','P4','P5','Q']
rows=[1,2,3,1,1,1,1,1,1]; isb=[False,True,True]+[False]*6
links=[('N0','T'),('N0','P1'),('P1','P2'),('P2','P3'),('P3','P4'),('P4','P5'),('P5','Q'),('S','Q')]
run('interleave',units,rows,isb,links)
# l5loop: B T S n1 n4
run('l5loop',['B','T','S','n1','n4'],[3,4,2,1,1],[True,True,True,False,False],[('n1','T'),('n4','B'),('S','T')])

# fuzz: compare level rows between pair modes; rounds vs pairs
random.seed(int(sys.argv[1]) if len(sys.argv)>1 else 7)
better=worse=same=0; maxex={}; viol=0
hist={}
for trial in range(60000):
    n=random.randint(3,11)
    isblock=[random.random()<0.7 for _ in range(n)]
    rows=[random.randint(1,5) if isblock[i] else 1 for i in range(n)]
    order=list(range(n)); random.shuffle(order)
    links=set()
    for _ in range(random.randint(1,2*n)):
        i,j=sorted(random.sample(range(n),2))
        links.add((order[i],order[j]))
    links=sorted(links)
    nb=sum(isblock); pairs=nb*(nb-1)//2
    rk=l5(n,rows,isblock,links,'cons','cons','key')
    rs=l5(n,rows,isblock,links,'cons','cons','start')
    rl=l5(n,rows,isblock,links,'cons','links','start')
    for r in (rk,rs,rl):
        assert r[0]=='ok', (r, rows, isblock, links)
        ex=r[2]-r[3]   # passes minus constraints added
        hist[ex]=hist.get(ex,0)+1
        if r[2] > pairs+2: viol+=1
    hk=max(rk[1][i]+rows[i] for i in range(n)); hs=max(rs[1][i]+rows[i] for i in range(n))
    if hs<hk: better+=1
    elif hs>hk: worse+=1
    else: same+=1
print('start-vs-key total rows: better',better,'worse',worse,'same',same)
print('passes minus added constraints histogram',hist,'passes>pairs+2:',viol)
