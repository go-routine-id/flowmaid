import sys
cols=sys.argv[1].split(',')
files=sys.argv[2:]
def load(f):
    rows=[l.rstrip('\n').split('\t') for l in open(f) if l.strip()]
    h=rows[0]
    return h,{(r[0],r[1]):r for r in rows[1:]}
data=[load(f) for f in files]
keys=sorted(set().union(*[set(d[1]) for d in data]))
def val(d,k,c):
    h,rows=d
    if c not in h or k not in rows: return None
    v=rows[k][h.index(c)]
    try: return float(v)
    except: return None
print('%-28s'%'file/mode', ' '.join('%-24s'%c for c in cols))
for k in keys:
    vals=[[val(d,k,c) for d in data] for c in cols]
    if all(len(set(v))<=1 for v in vals) and '-a' not in sys.argv[0:0]: 
        continue
    def fmt(v): return '/'.join('-' if x is None else ('%g'%round(x,2)) for x in v)
    print('%-28s'%('%s %s'%k), ' '.join('%-24s'%fmt(v) for v in vals))
