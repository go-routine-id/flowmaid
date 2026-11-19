import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t')); h=rows[0]
    return h,[dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)]
for name in ['bulkfc0','bulkst']:
    S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b11/'
    hh,a=load(S+f'bulk_{name}_head.tsv'); hn,b=load(S+f'bulk_{name}_now.tsv')
    ta={r['mode']:r for r in a if r[hh[0]]=='TOTAL'}; tb={r['mode']:r for r in b if r[hn[0]]=='TOTAL'}
    cols=['edge_node_pairs','edge_frame_pairs','edge_edge_x','x_drawn','overlap_pairs','tight_pairs','defects','lbl_node','lbl_lbl','lbl_edge','lbl_arrow','shared_ports','wiggles','hug_runs','border_runs','band_pairs','xgap_max','dash_edges','end_kink','end_tilt','end_gaps','lat_wiggles','own_frame_x','ink_out','idiom_ortho','xhops','fan_x','fan_uneven']
    print('==',name,len(a)//2,'files')
    for m in ['static','routed']:
        print(' ',m,' '.join(f"{c}:{ta[m].get(c,'-')}->{tb[m].get(c,'-')}" for c in cols if ta[m].get(c)!=tb[m].get(c)))
    # per-file regressions in defects / x_drawn
    ka={(r[hh[0]],r['mode']):r for r in a}; worse=[];better=0
    for r in b:
        k=(r[hn[0]],r['mode']); o=ka.get(k)
        if not o or k[0]=='TOTAL': continue
        for c in ['defects','x_drawn','overlap_pairs','edge_node_pairs','edge_frame_pairs','lbl_edge','tight_pairs']:
            d=float(r[c])-float(o[c])
            if d>0: worse.append((k,c,o[c],r[c]))
            elif d<0: better+=1
    print('  per-file worse cells:',len(worse),' better cells:',better)
    for w in worse[:40]: print('   ',w)
