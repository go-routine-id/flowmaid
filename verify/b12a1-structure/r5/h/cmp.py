import sys,csv
def load(p):
    rows={}
    with open(p) as f:
        r=csv.reader(f,delimiter='\t'); h=next(r)
        for x in r:
            if x[0]=='TOTAL': continue
            rows[(x[0],x[1])]=dict(zip(h,x))
    return rows
cols=sys.argv[3].split(',')
a=load(sys.argv[1]); b=load(sys.argv[2])
for k in sorted(b):
    if len(sys.argv)>4 and k[1]!=sys.argv[4]: continue
    pa=a.get(k,{})
    s='  '.join(f"{c}={pa.get(c,'?')}->{b[k].get(c,'?')}" for c in cols)
    print(f"{k[0]:32s} {k[1]:6s} {s}")
