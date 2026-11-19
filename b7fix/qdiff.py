import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; out={}
    for r in rows[1:]:
        if len(r)<3: continue
        out[(r[0],r[1])]=dict(zip(h,r))
    return h,out
h,a=load(sys.argv[1]); _,b=load(sys.argv[2])
keys=['edge_node_x','edge_frame_x','edge_edge_x','overlap_pairs','node_near','min_node_gap','lbl_node','lbl_lbl','lbl_chip','lbl_edge','lbl_arrow','shared_ports','wiggles','backtrack','frame_side','center_off','width','height','defects','corner_touch','border_run']
for k in sorted(set(a)|set(b)):
    ra=a.get(k); rb=b.get(k)
    if not ra or not rb: print(k,'missing'); continue
    d=[f"{x}:{ra[x]}->{rb[x]}" for x in keys if x in ra and ra[x]!=rb[x]]
    if d: print(k[0],k[1],' '.join(d))
