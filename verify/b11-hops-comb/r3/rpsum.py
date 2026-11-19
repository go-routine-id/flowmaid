import sys,glob,collections
d=sys.argv[1]
def load(w):
    fr=collections.defaultdict(list)
    for p in sorted(glob.glob(f'{d}/dump-{w}-*.tsv')):
        for l in open(p):
            f,n,node,ms=l.rstrip().split('\t'); fr[(f,int(n),node)].append(float(ms))
    return fr
pre,now=load('pre'),load('now')
files=sorted({k[0] for k in now})
for f in files:
    ks=[k for k in now if k[0]==f]
    mn=lambda fr,k: min(fr[k]) if k in fr else float('nan')
    wp=max(ks,key=lambda k:mn(pre,k)); wn=max(ks,key=lambda k:mn(now,k))
    over=[(k,now[k],pre.get(k)) for k in ks if min(now[k])>6.0]
    anyover=[(k,now[k],pre.get(k)) for k in ks if max(now[k])>6.0]
    mp=sum(mn(pre,k) for k in ks)/len(ks); mnw=sum(mn(now,k) for k in ks)/len(ks)
    print(f"{f}\tframes={len(ks)}\tpre worst(min-of-runs)={mn(pre,wp):.2f} [{wp[1]} {wp[2]}]\tnow worst={mn(now,wn):.2f} [{wn[1]} {wn[2]}]\tmean {mp:.3f}->{mnw:.3f}\tover6(min)={len(over)} over6(any run)={len(anyover)}")
    for k,a,b in anyover[:12]:
        print('    ',k[1],k[2],'now',['%.2f'%x for x in a],'pre',['%.2f'%x for x in (b or [])])
