import sys, csv
# qdiff.py A.tsv B.tsv [C.tsv]: verdict columns per file/mode; print rows where any worse from A to last
cols_up = ["defects","x_drawn","lateral_x","fan_x","edge_chip","side_ends","edge_node_pairs","edge_frame_pairs","lbl_edge","lbl_chip","frame_side","node_frame"]
def load(p):
    rows = {}
    with open(p) as f:
        r = csv.DictReader(f, delimiter="\t")
        for row in r:
            rows[(row["file"], row["mode"])] = row
    return rows
ts = [load(p) for p in sys.argv[1:]]
A, Z = ts[0], ts[-1]
nw = nb = 0
for k in sorted(Z):
    if k not in A: print("MISSING in A", k); continue
    a, z = A[k], Z[k]
    bad = []; good = []
    for c in cols_up:
        if c not in z: continue
        va, vz = float(a[c]), float(z[c])
        if vz > va: bad.append(f"{c} " + "/".join(t[k][c] for t in ts))
        elif vz < va: good.append(f"{c} " + "/".join(t[k][c] for t in ts))
    ga, gz = float(a["min_edge_gap"]), float(z["min_edge_gap"])
    if gz < ga - 0.5: bad.append("min_edge_gap " + "/".join(t[k]["min_edge_gap"] for t in ts))
    elif gz > ga + 0.5: good.append("min_edge_gap " + "/".join(t[k]["min_edge_gap"] for t in ts))
    if bad: nw += 1; print("WORSE", k[0], k[1], "; ".join(bad), "| better:", "; ".join(good))
    elif good: nb += 1
print(f"worse {nw}, better-only {nb}, of {len(Z)}")
