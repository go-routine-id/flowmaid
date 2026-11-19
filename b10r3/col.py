import sys,csv
rows=list(csv.reader(sys.stdin,delimiter='\t')); h=rows[0]
cols=sys.argv[1].split(',')
for r in rows[1:]:
    if len(r)!=len(h): continue
    d=dict(zip(h,r)); print(d['file'],d['mode'],*[d[c] for c in cols])
