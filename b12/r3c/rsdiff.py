# rsdiff.py <run>/dirs/dirs.tsv: copies whose routed row differs from the static row in the verdict columns
import sys
cols=["defects","x_drawn","lateral_x","fan_x","edge_chip","side_ends","min_edge_gap"]
rows={}
hdr=None
for l in open(sys.argv[1]):
    f=l.rstrip('\n').split('\t')
    if hdr is None: hdr=f; continue
    d=dict(zip(hdr,f)); rows[(d['file'],d['mode'])]=d
n=0
for (fi,m),d in sorted(rows.items()):
    if m!='static': continue
    r=rows.get((fi,'routed'))
    if not r: continue
    diff=[f"{c} {d[c]}->{r[c]}" for c in cols if abs(float(d[c])-float(r[c]))>1e-6]
    if diff: n+=1; print(fi, "; ".join(diff))
print("copies with routed != static:", n)
