import math,sys
sys.path.insert(0,'.')
from geo import edges_of, segx
for f in sys.argv[1:]:
    E=edges_of(f); hist={}; multi=[]
    for i in range(len(E)):
        for j in range(i+1,len(E)):
            P=E[i][1];Q=E[j][1];found=[]
            for k in range(len(P)-1):
                a,b=P[k],P[k+1]
                for m in range(len(Q)-1):
                    c,d=Q[m],Q[m+1]
                    if max(c[0],d[0])<min(a[0],b[0]) or min(c[0],d[0])>max(a[0],b[0]) or max(c[1],d[1])<min(a[1],b[1]) or min(c[1],d[1])>max(a[1],b[1]): continue
                    x=segx(a,b,c,d)
                    if x and all(math.dist(x,q)>4 for q in found): found.append(x)
            ends=[P[0],P[-1],Q[0],Q[-1]]
            found=[q for q in found if min(math.dist(q,e) for e in ends)>10]
            n=len(found); hist[n]=hist.get(n,0)+1
            if n>=2: multi.append((E[i][0],E[j][0],n))
    print(f.split('/')[-1],'pairs by #crossings',sorted(hist.items()),'extra crossings from multi-crossing pairs',sum(n-1 for _,_,n in multi))
    if 'flowmaid' in f: print(sorted(multi,key=lambda x:-x[2])[:12])
