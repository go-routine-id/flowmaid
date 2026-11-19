import sys,re,collections
def load(p):
    d=collections.defaultdict(dict); f=None
    for line in open(p):
        if line.startswith('  '):
            m=re.match(r'  (S|R) X (\S+) x (\S+) : (\d+)',line)
            d[(f,m.group(1))][(m.group(2),m.group(3))]=int(m.group(4))
        elif not line.startswith('TOTAL'):
            f=line.split('\t')[0].split('/')[-1]; 
    return d
a=load(sys.argv[1]); b=load(sys.argv[2]); mode=sys.argv[3]
cat=collections.Counter(); catn=collections.Counter()
def kind(p):
    x,y=p
    n=('note' in x)+('note' in y)
    return 'note' if n else 'edge'
keys=set(k for k in a if k[1]==mode)|set(k for k in b if k[1]==mode)
for k in keys:
    pa=a.get(k,{}); pb=b.get(k,{})
    for p in set(pa)|set(pb):
        dlt=pb.get(p,0)-pa.get(p,0)
        if dlt>0: cat[kind(p)+'+']+=dlt
        if dlt<0: cat[kind(p)+'-']+=dlt
        if pb.get(p,0)>=2: catn[kind(p)+' dbl_now']+=1
        if pa.get(p,0)>=2: catn[kind(p)+' dbl_pre']+=1
print(cat); print(catn)
