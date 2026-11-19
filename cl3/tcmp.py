import csv,sys
O=sys.argv[1]; A=sys.argv[2]; B=sys.argv[3]
def load(p): return {r[0]:[float(x) for x in r[1:]] for r in list(csv.reader(open(O+p),delimiter='\t'))[1:]}
a=load(A+'1.tsv'); a2=load(A+'2.tsv'); b=load(B+'1.tsv'); b2=load(B+'2.tsv')
worst=[]; over=0
for k in a:
    pa=[min(x,y) for x,y in zip(a[k],a2[k])]; pb=[min(x,y) for x,y in zip(b[k],b2[k])]
    for i in (1,2):
        if pb[i] > max(1.25*pa[i], pa[i]+1.0): over+=1
    worst.append((max(pb[i]/pa[i] for i in range(3)), k.split('/')[-1], pa, pb))
worst.sort(reverse=True)
for w in worst[:8]: print('%.2fx %s pre %s now %s'%w)
for i,n in enumerate(['scene','route','drag']):
    print(n,'sum pre %.1f now %.1f'%(sum(min(a[k][i],a2[k][i]) for k in a), sum(min(b[k][i],b2[k][i]) for k in a)), 'max pre %.2f now %.2f'%(max(min(a[k][i],a2[k][i]) for k in a), max(min(b[k][i],b2[k][i]) for k in a)))
print('route/drag over budget max(1.25x, +1ms):', over, 'of', 2*len(a))
