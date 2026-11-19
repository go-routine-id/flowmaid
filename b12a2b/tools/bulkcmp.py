import csv,sys
# bulkcmp.py <a.tsv> <b.tsv>: totals and per-file changes of key columns, static and routed
cols=['x_drawn','chip_crowd','fan_x','lateral_x','side_ends','ortho_edges','bends_max','defects','edge_chip','min_edge_gap','edge_frame_pairs','overlap_pairs']
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; out={}
    for r in rows[1:]:
        if len(r)!=len(h) or r[0]=='TOTAL': continue
        d=dict(zip(h,r)); out[(d['file'],d['mode'])]=d
    return out
a=load(sys.argv[1]); b=load(sys.argv[2])
def f(x):
    try: return float(x)
    except: return 0.0
for mode in ['static','routed']:
    tot={c:[0,0] for c in cols}; ch=[]
    for k in a:
        if k[1]!=mode or k not in b: continue
        for c in cols:
            if c not in a[k]: continue
            tot[c][0]+=f(a[k][c]); tot[c][1]+=f(b[k][c])
        diffs=[(c,a[k][c],b[k][c]) for c in cols if c in a[k] and a[k][c]!=b[k][c]]
        if diffs: ch.append((k[0],diffs))
    print(mode, ' '.join(f"{c} {tot[c][0]:g}->{tot[c][1]:g}" for c in cols if c!='min_edge_gap'))
    print('  files changed:',len(ch))
    for fn,d in ch[:25]: print('   ',fn,' '.join(f"{c} {x}->{y}" for c,x,y in d))
missing=[k for k in a if k not in b]
print('missing',len(missing))
