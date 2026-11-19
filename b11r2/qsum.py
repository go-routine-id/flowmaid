#!/usr/bin/env python3
# qsum.py <quality.tsv> [other.tsv]: totals per mode of the columns that matter; with two files, per-file diffs
import sys, csv
COLS = ["fan_x","x_drawn","edge_edge_x","defects","overlap_pairs","tight_pairs","band_pairs","fan_uneven","edge_node_pairs","edge_frame_pairs","lbl_edge","wiggles","end_kink","xhops","border_runs","hug_runs","edge_chip"]
def load(p):
    rows = {}
    with open(p) as f:
        r = csv.reader(f, delimiter="\t")
        hdr = next(r)
        for row in r:
            if len(row) < len(hdr) or row[0] in ("TOTAL","total") or row[0].startswith("#"): continue
            d = dict(zip(hdr, row))
            rows[(d["file"], d["mode"])] = d
    return hdr, rows
def tot(rows, mode, c):
    s = 0.0
    for (f, m), d in rows.items():
        if m == mode and c in d:
            try: s += float(d[c])
            except: pass
    return s
h, a = load(sys.argv[1])
cols = [c for c in COLS if c in h]
if len(sys.argv) == 2:
    for mode in ("static","routed"):
        n = sum(1 for (f,m),d in a.items() if m==mode)
        nf = sum(1 for (f,m),d in a.items() if m==mode and float(d.get("fan_x",0))>0)
        print(mode, "files", n, "fan_x>0:", nf, " ".join(f"{c}={tot(a,mode,c):g}" for c in cols))
else:
    h2, b = load(sys.argv[2])
    cols = [c for c in cols if c in h2]
    for mode in ("static","routed"):
        nf = lambda rows: sum(1 for (f,m),d in rows.items() if m==mode and float(d.get("fan_x",0))>0)
        print(mode, "fan_x>0 files:", nf(a), "->", nf(b), " ".join(f"{c}={tot(a,mode,c):g}->{tot(b,mode,c):g}" for c in cols))
    worse = []
    for k in sorted(a):
        if k not in b: continue
        for c in cols:
            try: x, y = float(a[k][c]), float(b[k][c])
            except: continue
            if y > x and c not in ("xhops",): worse.append((k, c, x, y))
    for w in worse[:int(sys.argv[3]) if len(sys.argv) > 3 else 60]: print("  WORSE", w[0][0], w[0][1], w[1], f"{w[2]:g}->{w[3]:g}")
    print("  worse rows:", len(worse))
