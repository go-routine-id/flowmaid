import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h) and r[0]!='TOTAL'}
# base for each file: from A_new if present else head
head=load(sys.argv[1]); a=load(sys.argv[2]); b=load(sys.argv[3]); cols=sys.argv[4].split(',')
for mode in ['static','routed']:
    ks=[k for k in b if k[1]==mode]
    base={k:(a[k] if k in a else head[k]) for k in ks}
    diff=[k for k in ks if base[k]['width']!=b[k]['width'] or base[k]['height']!=b[k]['height']]
    out=[f"{mode}: differ={len(diff)}"]
    for c in cols:
        sa=sum(float(base[k][c]) for k in diff); sb=sum(float(b[k][c]) for k in diff)
        out.append(f"{c} {sa:g}->{sb:g}")
    print('  '.join(out))
    if len(sys.argv)>5:
        for k in diff: print('   ',k[0], ' '.join(f"{c}:{base[k][c]}->{b[k][c]}" for c in cols), base[k]['width'],'->',b[k]['width'], base[k]['height'],'->',b[k]['height'])
