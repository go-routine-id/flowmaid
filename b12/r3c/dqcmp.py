import sys
def load(p):
    d={}
    for l in open(p):
        if l.startswith(' '): continue
        f=l.rstrip('\n').split('\t')
        if len(f)<4: continue
        d[(f[0],f[2])]={k:float(v) for k,v in (x.split('=') for x in f[3:])}
    return d
a,b=load(sys.argv[1]),load(sys.argv[2])
keys=["defects","x_drawn","edge_node_x","fan_x","edge_frame_pairs","lateral_x","edge_chip","overlap_pairs"]
for kind in ["1px","mid","far"]:
    ta={k:sum(v[k] for (f,d),v in a.items() if d==kind) for k in keys}
    tb={k:sum(v[k] for (f,d),v in b.items() if d==kind) for k in keys}
    print(kind, "  ".join(f"{k} {ta[k]:g}->{tb[k]:g}{' WORSE' if tb[k]>ta[k] and k in keys[:4] else ''}" for k in keys))
    for (f,d),v in sorted(b.items()):
        if d!=kind or (f,d) not in a: continue
        w=[f"{k} {a[(f,d)][k]:g}->{v[k]:g}" for k in keys[:4] if v[k]>a[(f,d)][k]]
        if w: print("   ",f,"; ".join(w))
