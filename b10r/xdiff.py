import collections,sys
def load(f,tag='static'):
    d={}
    for l in open(f):
        p=l.split()
        if p[1]!=tag: continue
        d[(p[2],p[4])]=int(p[5])
    return d
tag=sys.argv[3] if len(sys.argv)>3 else 'static'
a=load(sys.argv[1],tag); b=load(sys.argv[2],tag)
print('total',sum(a.values()),sum(b.values()))
per=collections.Counter()
for k in set(a)|set(b):
    dlt=b.get(k,0)-a.get(k,0)
    per[k[0]]+=dlt; per[k[1]]+=dlt
print(' '.join(f"{e}:{v}" for e,v in per.most_common(14)))
print(' '.join(f"{e}:{v}" for e,v in sorted(per.items(), key=lambda x:x[1])[:10]))
