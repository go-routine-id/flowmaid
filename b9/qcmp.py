import csv,sys,statistics as st
def load(p):
    r=list(csv.DictReader(open(p),delimiter='\t'))
    return {(x['file'],x['mode']):x for x in r}
a=load(sys.argv[1]); b=load(sys.argv[2])
cols=['defects','edge_node_x','edge_frame_x','edge_edge_x','overlap_pairs','lbl_node','lbl_lbl','lbl_edge','lbl_arrow','node_overlap','node_frame','frame_overlap','own_frame_x','end_cross','backtrack','wiggles','area_per_node','width','height','aspect','route_ms','drag_ms','scene_ms','deterministic','x_drawn','ink_out','end_gaps','corner_touch','border_runs','hug_runs','tight_pairs']
for mode in ['static','routed']:
    print('==',mode)
    keys=[k for k in b if k[1]==mode and k in a]
    for c in cols:
        if c not in b[keys[0]]: continue
        try:
            va=[float(a[k][c]) for k in keys]; vb=[float(b[k][c]) for k in keys]
        except: continue
        worse=sum(1 for x,y in zip(va,vb) if y>x+1e-9); better=sum(1 for x,y in zip(va,vb) if y<x-1e-9)
        print(f"{c:15s} sum {sum(va):10.1f} -> {sum(vb):10.1f}  worse {worse:3d} better {better:3d}")
