import sys
cols=['edge_node_pairs','edge_frame_pairs','edge_edge_x','overlap_len','tight_len','hug_len','edge_chip','lbl_node','lbl_lbl','lbl_edge','lbl_arrow','defects','min_edge_gap','band_pairs','band_len','own_frame_len','x_drawn','lat_backtrack','end_tilt','wiggles','shared_ports']
def load(f):
    rows=[l.rstrip('\n').split('\t') for l in open(f)]
    h=rows[0]; return h,[r for r in rows[1:] if r[0]!='TOTAL']
out=[]
for f in sys.argv[1:]:
    h,rows=load(f)
    agg={}
    for mode in ['static','routed']:
        rs=[r for r in rows if r[1]==mode]
        for c in cols:
            i=h.index(c)
            vals=[float(r[i]) for r in rs]
            agg[(mode,c)]=min(vals) if c=='min_edge_gap' else sum(vals)
    out.append(agg)
print('%-16s'%'metric'+''.join('%22s'%f.split('/')[-1][:20] for f in sys.argv[1:]))
for mode in ['static','routed']:
    for c in cols:
        print('%-7s%-16s'%(mode,c)+''.join('%22.1f'%a[(mode,c)] for a in out))
