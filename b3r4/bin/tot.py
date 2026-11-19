import csv,sys
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return h,{(r[0],r[1]):r for r in rows[1:]}
h,a=load(sys.argv[1]); _,b=load(sys.argv[2])
prefix=sys.argv[3] if len(sys.argv)>3 else ''
cols=['defects','edge_edge_x','overlap_pairs','overlap_len','edge_node_x','edge_frame_x','lbl_edge','lbl_node','lbl_lbl','shared_ports','end_tilt','end_tilt_max','own_frame_x','band_pairs','x_drawn','end_kink','corner_touch','border_runs','width','height']
for mode in ['static','routed']:
    out=[]
    for c in cols:
        i=h.index(c)
        sa=sum(float(r[i]) for k,r in a.items() if k[1]==mode and k[0]!='TOTAL' and k[0].startswith(prefix))
        sb=sum(float(r[i]) for k,r in b.items() if k[1]==mode and k[0]!='TOTAL' and k[0].startswith(prefix))
        if abs(sa-sb)>1e-6: out.append(f"{c}:{sa:g}->{sb:g}")
    print(mode,' '.join(out))
