import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)}
a=load(sys.argv[1]); b=load(sys.argv[2])
cols=sys.argv[3].split(',') if len(sys.argv)>3 else ['defects','x_drawn','edge_frame_x','edge_node_x','edge_edge_x']
for mode in ['static','routed']:
    ks=[k for k in a if k[1]==mode and k in b]
    chg=[k for k in ks if (a[k]['width'],a[k]['height'])!=(b[k]['width'],b[k]['height'])]
    out=[f"{mode}: n={len(ks)} changed={len(chg)}"]
    for c in cols:
        sa=sum(float(a[k][c]) for k in ks); sb=sum(float(b[k][c]) for k in ks)
        worse=sum(1 for k in ks if float(b[k][c])>float(a[k][c])); better=sum(1 for k in ks if float(b[k][c])<float(a[k][c]))
        out.append(f"{c} {sa:g}->{sb:g} (+{worse}/-{better})")
    print('  '.join(out))
