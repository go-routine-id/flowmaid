import sys
cols=['file','mode','edge_node_x','edge_node_pairs','edge_frame_pairs','shared_ports','wiggles','frame_side','node_overlap','node_frame','frame_overlap','defects','route_ms','deterministic','min_edge_gap','x_drawn','own_frame_x','overlap_pairs','lbl_edge','scene_ms','drag_ms','drag_mean','drag_over']
for f in sys.argv[1:]:
    rows=[l.rstrip('\n').split('\t') for l in open(f)]
    h=rows[0]
    idx=[h.index(c) for c in cols if c in h]
    names=[c for c in cols if c in h]
    print('==',f.split('/')[-1])
    w=[max(len(names[k]),max(len(r[i]) for r in rows[1:] if i<len(r))) for k,i in enumerate(idx)]
    print(' '.join(n.ljust(w[k]) for k,n in enumerate(names)))
    for r in rows[1:]:
        print(' '.join(r[i].ljust(w[k]) for k,i in enumerate(idx)))
