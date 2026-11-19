import sys,csv
# cols.py perf.tsv col1,col2,... [other.tsv]
rows=list(csv.reader(open(sys.argv[1]),delimiter='\t'))
h=rows[0]; cols=sys.argv[2].split(',')
other=None
if len(sys.argv)>3:
    r2=list(csv.reader(open(sys.argv[3]),delimiter='\t')); h2=r2[0]
    other={(r[0],r[1]):dict(zip(h2,r)) for r in r2[1:] if len(r)==len(h2)}
print('file\tmode\t'+'\t'.join(cols))
for r in rows[1:]:
    if len(r)!=len(h): continue
    d=dict(zip(h,r))
    if other is not None:
        o=other.get((r[0],r[1]),{})
        print(f"{r[0][:14]:14}\t{r[1]}\t"+'\t'.join((f"{o.get(c,'?')}->{d.get(c,'?')}" if o.get(c)!=d.get(c) else f"{d.get(c)}") for c in cols))
    else:
        print(f"{r[0][:14]:14}\t{r[1]}\t"+'\t'.join(d.get(c,'?') for c in cols))
