import csv,sys
def load(f):
    rows=list(csv.reader(open(f),delimiter='\t'))
    h=rows[0]
    return h,{(r[0],r[1]):r for r in rows[1:]}
h,a=load(sys.argv[1]); h2,b=load(sys.argv[2])
cols=sys.argv[3].split(',')
print('file\tmode\t'+'\t'.join(cols))
for k in a:
    if k not in b: continue
    out=[]
    for c in cols:
        va=a[k][h.index(c)] if c in h else '?'
        vb=b[k][h2.index(c)] if c in h2 else '?'
        out.append(va if va==vb else f'{va}->{vb}')
    print(k[0]+'\t'+k[1]+'\t'+'\t'.join(out))
