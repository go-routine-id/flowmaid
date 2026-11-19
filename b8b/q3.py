import csv,sys
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t')); h=rows[0]
    return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h) and r[0]!='TOTAL'}
d=sys.argv[1]; new=sys.argv[2] if len(sys.argv)>2 else 'v2'
a=load(d+'/q.035.tsv'); b=load(d+f'/q.{new}.tsv'); c=load(d+'/q.034.tsv')
S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b8b/'
w26=open(S+'worse26.static.txt').read().split(); b235=open(S+'better235.static.txt').read().split()
cols=['defects','edge_frame_x','edge_node_x','x_drawn','edge_chip']
for mode in ('static','routed'):
    ks=[k for k in b if k[1]==mode and k in a and k in c]
    tot=lambda m,col: sum(float(m[k][col]) for k in ks)
    print(mode, len(ks), ' '.join(f"{col}: 034={tot(c,col):.0f} 035={tot(a,col):.0f} new={tot(b,col):.0f}" for col in cols))
    for col in ('edge_frame_x','defects'):
        worse=[k[0] for k in ks if float(b[k][col])>float(a[k][col])]
        better=[k[0] for k in ks if float(b[k][col])<float(a[k][col])]
        print(f"  vs035 {col}: worse {len(worse)} better {len(better)}  worse: "+' '.join(f"{n}({a[(n,mode)][col]}->{b[(n,mode)][col]})" for n in worse[:30]))
    # 26 worse (034->035 on efx): status now vs 034
    if mode=='static':
        st=[(n, c[(n,mode)]['edge_frame_x'], a[(n,mode)]['edge_frame_x'], b[(n,mode)]['edge_frame_x']) for n in w26 if (n,mode) in b]
        fixed=[s for s in st if float(s[3])<=float(s[1])]
        print(f"  26-worse: fixed(<=034) {len(fixed)}/{len(st)}; still worse:", [s for s in st if float(s[3])>float(s[1])])
        bl=[(n, c[(n,mode)]['edge_frame_x'], b[(n,mode)]['edge_frame_x']) for n in b235 if (n,mode) in b]
        lost=[s for s in bl if float(s[2])>=float(s[1])]
        print(f"  better-set: still better than 034 {len(bl)-len(lost)}/{len(bl)}; lost:", lost[:20])
