#!/usr/bin/env bash
# cmp.sh <quality-tsv> : selected harness columns beside the start values (static rows)
B=$(cd "$(dirname "$0")/.." && pwd)
python3 - "$1" "$B/start/quality-start-b121.tsv" <<'PY'
import sys,csv
def load(p):
    rows={}
    with open(p) as f:
        r=csv.reader(f,delimiter='\t'); h=next(r)
        for row in r:
            d=dict(zip(h,row)); rows[(d[h[0]],d['mode'])]=d
    return rows,h
new,h=load(sys.argv[1]); old,_=load(sys.argv[2])
cols=['x_drawn','edge_frame_x','edge_node_x','defects','wiggles','frame_side','shared_ports','lateral_x','up_lanes','side_ends','fan_x','lbl_edge','chip_crowd','scene_ms']
for mode in ('static','routed'):
    print('==',mode)
    print('%-34s'%'name',' '.join('%12s'%c for c in cols))
    for (n,m),d in sorted(new.items()):
        if m!=mode: continue
        o=old.get((n,m),{})
        print('%-34s'%n[:34],' '.join('%12s'%('%s|%s'%(d.get(c,'?'),o.get(c,'?'))) for c in cols))
PY
