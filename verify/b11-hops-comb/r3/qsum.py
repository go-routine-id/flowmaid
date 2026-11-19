import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return h,{(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if r[0]!='TOTAL'}
hb,b=load(sys.argv[1]); hc,c=load(sys.argv[2])
cols=sys.argv[3].split(',')
for mode in ['static','routed']:
    print('==',mode)
    for col in cols:
        if col not in hc: continue
        tb=sum(float(v.get(col,0) or 0) for k,v in b.items() if k[1]==mode) if col in hb else float('nan')
        tc=sum(float(v[col] or 0) for k,v in c.items() if k[1]==mode)
        worse=[(k[0],b[k].get(col),c[k][col]) for k in c if k[1]==mode and k in b and col in hb and float(c[k][col] or 0)>float(b[k][col] or 0)+1e-9]
        better=sum(1 for k in c if k[1]==mode and k in b and col in hb and float(c[k][col] or 0)<float(b[k][col] or 0)-1e-9)
        nz=sum(1 for k in c if k[1]==mode and float(c[k][col] or 0)>0)
        print(f"  {col}: {tb:g} -> {tc:g}   files>0 now: {nz}   worse in {len(worse)} better in {better}", worse[:8])
