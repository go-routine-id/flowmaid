import sys,collections
def load(p):
    d={}
    for l in open(p):
        f=l.rstrip('\n').split('\t')
        if len(f)<3 or f[1]!='static': continue
        kv=dict(x.split('=',1) for x in f[2:] if '=' in x)
        d[f[0]]=kv
    return d
off=load(sys.argv[1]); on=load(sys.argv[2]); prev=load(sys.argv[3]) if len(sys.argv)>3 else None
kinds=['enx','sp','aos','wig','fside','nov','nfr','fov','own']
bm=[k for k in on if on[k].get('bm')=='1']
print('inputs',len(on),'block mode',len(bm))
def tot(d,keys,c): return sum(int(d[k].get(c,0)) for k in keys if k in d)
for c in ['hard']+kinds+['x','fan','ovl','efp']:
    print(f"{c:6s} start {tot(off,bm,c):6d}  now {tot(on,bm,c):6d}" + (f"  prev {tot(prev,bm,c):6d}" if prev else ''))
worse=[k for k in bm if int(on[k]['hard'])>int(off[k]['hard'])]
print('files with more hard zeros than start:',len(worse))
cnt=collections.Counter()
for k in worse:
    for c in kinds:
        if int(on[k][c])>int(off[k][c]): cnt[c]+=1
print(' by kind:',dict(cnt))
if prev:
    pw=[k for k in bm if int(prev[k]['hard'])>int(off[k]['hard'])]
    print('prev worse files',len(pw),' new now:',sorted(set(worse)-set(pw)),' gone:',sorted(set(pw)-set(worse)))
    ch=[k for k in bm if on[k]!=prev.get(k)]
    print('files whose row changed vs prev:',len(ch))
    for k in ch[:40]:
        diff={c:(prev[k].get(c),on[k].get(c)) for c in on[k] if prev[k].get(c)!=on[k].get(c)}
        print('  ',k,diff)
open(sys.argv[2]+'.worse','w').write('\n'.join(sorted(worse))+'\n')
