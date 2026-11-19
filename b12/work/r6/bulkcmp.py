import sys,collections
def load(p):
    d={}
    for l in open(p):
        f=l.rstrip('\n').split('\t')
        if len(f)<3 or f[1]=='PANIC': continue
        kv=dict(x.split('=',1) for x in f[2:] if '=' in x)
        d[(f[0],f[1])]=kv
    return d
a=load(sys.argv[1]); b=load(sys.argv[2])
for mode in ['static','routed']:
    keys=[k for k in b if k[1]==mode and k in a and b[k].get('bm')=='1']
    tot=collections.Counter(); totb=collections.Counter()
    worse=collections.Counter(); better=collections.Counter()
    for k in keys:
        for c in ['hard','efp','x','fan','ovl','def','own','nfr','sp','aos','enx']:
            x=float(a[k][c]); y=float(b[k][c]); tot[c]+=x; totb[c]+=y
            if y>x: worse[c]+=1
            if y<x: better[c]+=1
    print(mode, len(keys),'block-mode inputs')
    for c in ['hard','efp','x','fan','ovl','def','own','nfr','sp','aos','enx']:
        print('  %-5s prev %6g now %6g   worse on %3d better on %3d'%(c,tot[c],totb[c],worse[c],better[c]))
panics=[l.split('\t')[0] for l in open(sys.argv[2]) if '\tPANIC' in l]
print('panics', panics)
