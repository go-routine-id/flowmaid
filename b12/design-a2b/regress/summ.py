# summ.py: totals per family and config over the quality TSVs in m/ (static and routed),
# and the per-file changes between configs.
import csv,sys,os
M='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/regress/m'
fams=['bulkst','bulkfc','all','dirs','dirsx','stress']
cfgs=['pre','t78','on','r1']
cols=['x_drawn','edge_edge_x','chip_crowd','fan_x','lateral_x','side_ends','ortho_edges','border_runs','border_len','defects','wiggles','backtrack','bends_max','lat_wiggles','frame_side','edge_chip','lbl_crowd','overlap_pairs','min_edge_gap']
def load(f,c):
    p=f'{M}/{f}-{c}.tsv'
    if not os.path.exists(p): return None
    rows=list(csv.reader(open(p),delimiter='\t')); h=rows[0]
    return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if r and r[0]!='TOTAL'}
data={(f,c):load(f,c) for f in fams for c in cfgs}
def tot(d,mode,col):
    return sum(float(r[col]) for k,r in d.items() if k[1]==mode)
mode=sys.argv[1] if len(sys.argv)>1 else 'static'
print(f'## totals ({mode})')
print('family | config | files | '+' | '.join(cols))
for f in fams:
    for c in cfgs:
        d=data[(f,c)]
        if d is None: continue
        n=sum(1 for k in d if k[1]==mode)
        vals=[]
        for col in cols:
            t=tot(d,mode,col)
            vals.append(f'{t:.0f}' if col not in('border_len',) else f'{t:.1f}')
        print(f'{f} | {c} | {n} | '+' | '.join(vals))
