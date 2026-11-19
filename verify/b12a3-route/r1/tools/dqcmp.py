import sys
def load(p):
    d={}
    for line in open(p):
        parts=line.rstrip('\n').split('\t')
        if len(parts)<4: continue
        d[(parts[0],parts[2])]={k:float(v) for k,v in (x.split('=') for x in parts[3:])}
    return d
pre=load(sys.argv[1]); head=load(sys.argv[2])
keys=['defects','x_drawn','edge_node_x','fan_x','edge_frame_pairs','lateral_x','edge_chip','overlap_pairs']
for drag in ('1px','mid','far'):
    tp={k:0 for k in keys}; th={k:0 for k in keys}
    for (n,d),v in pre.items():
        if d!=drag: continue
        for k in keys: tp[k]+=v[k]; th[k]+=head[(n,d)][k]
    print(drag,' '.join(f"{k} {tp[k]:g}->{th[k]:g}" for k in keys))
print('per-file worse (defects/x_drawn/edge_node_x/fan_x):')
for (n,d),v in sorted(pre.items()):
    w=[f"{k} {v[k]:g}->{head[(n,d)][k]:g}" for k in ('defects','x_drawn','edge_node_x','fan_x') if head[(n,d)][k]>v[k]]
    if w: print(' ',n,d,', '.join(w))
