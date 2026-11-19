import csv,sys
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t')); h=rows[0]
    return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h) and r[0]!='TOTAL'}
d=sys.argv[1]
a=load(d+'/q.035.tsv'); b=load(d+'/q.new.tsv'); c=load(d+'/q.034.tsv')
cols=['defects','edge_frame_x','edge_node_x','x_drawn','edge_chip']
for mode in ('static','routed'):
    ks=[k for k in b if k[1]==mode and k in a]
    tot=lambda m,col: sum(float(m[k][col]) for k in ks if k in m)
    print(mode, len(ks), ' '.join(f"{col}: 034={tot(c,col):.0f} 035={tot(a,col):.0f} new={tot(b,col):.0f}" for col in cols))
    for col in ('edge_frame_x','defects'):
        worse=[k[0] for k in ks if float(b[k][col])>float(a[k][col])]
        better=[k[0] for k in ks if float(b[k][col])<float(a[k][col])]
        print(f"  {col}: worse {len(worse)} better {len(better)}")
        if len(sys.argv)>2: print('   worse:',' '.join(f"{n}({a[(n,mode)][col]}->{b[(n,mode)][col]})" for n in worse[:40]))
