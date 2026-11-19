import sys, math, re
base = sys.argv[1]
readme = sys.argv[2]
def tsv(p):
    rows = {}
    with open(p) as f:
        hdr = f.readline().rstrip('\n').split('\t')
        for line in f:
            c = line.rstrip('\n').split('\t')
            if not c or not c[0]: continue
            rows[c[0]] = dict(zip(hdr, c))
    return rows
f39 = tsv(base + '/bench039/flowmaid.tsv'); m39 = tsv(base + '/bench039/mermaid.tsv')
f38 = tsv(base + '/bench038/flowmaid.tsv'); m38 = tsv(base + '/bench038/mermaid.tsv')
names = list(f39)
assert set(names) == set(m39) == set(f38) == set(m38), "name sets differ"
print("n =", len(names))
def gm(xs): return math.exp(sum(math.log(x) for x in xs)/len(xs))

# (1) table cells: README vs compare.md and vs tsv recomputation
def table(path, start_pat):
    rows = {}
    on = False
    for i, line in enumerate(open(path), 1):
        if line.startswith('| diagram | type |'): on = True; continue
        if on:
            if not line.startswith('|'):
                if rows: break
                continue
            if line.startswith('|---'): continue
            c = [x.strip() for x in line.strip().strip('|').split('|')]
            rows[c[0]] = (i, c)
    return rows
tr = table(readme, None)
tc = table(base + '/bench039/compare.md', None)
print("README rows", len(tr), "compare rows", len(tc))
bad = 0
for n in names:
    if tr[n][1] != tc[n][1]:
        bad += 1; print("DIFF README vs compare.md", n, tr[n], tc[n])
print("README vs compare.md cell mismatches:", bad)
# vs tsv
def r(x, d): 
    # round half up on decimal repr
    from decimal import Decimal, ROUND_HALF_UP
    return str(Decimal(str(x)).quantize(Decimal(1).scaleb(-d), rounding=ROUND_HALF_UP))
bad = 0
for n in names:
    fm = float(f39[n]['median_ms']); mm = float(m39[n]['median_ms'])
    exp = [n, f39[n]['type'], r(f39[n]['median_ms'],2), r(m39[n]['median_ms'],1), f"{round(mm/fm)}×",
           f"{int(f39[n]['svg_bytes'])/1024:.1f}", f"{int(m39[n]['svg_bytes'])/1024:.1f}"]
    exp1000 = [f"{int(f39[n]['svg_bytes'])/1000:.1f}", f"{int(m39[n]['svg_bytes'])/1000:.1f}"]
    got = tr[n][1]
    if got != exp:
        bad += 1; print("DIFF vs tsv", n, "got", got, "exp(1024)", exp, "exp(1000)", exp1000, "ratio", mm/fm)
print("README vs tsv mismatches (KB=1024):", bad)

# (2) geomean
sp39 = [float(m39[n]['median_ms'])/float(f39[n]['median_ms']) for n in names]
sp38 = [float(m38[n]['median_ms'])/float(f38[n]['median_ms']) for n in names]
print("geomean 0.39: %.3f   0.38: %.3f" % (gm(sp39), gm(sp38)))
# (3)
sub = [n for n in names if float(f39[n]['median_ms']) < 1.0]
print("under 1 ms (0.39):", len(sub), "not:", [n for n in names if n not in sub])
sub38 = [n for n in names if float(f38[n]['median_ms']) < 1.0]
print("under 1 ms (0.38):", len(sub38))
sz = {n: int(m39[n]['svg_bytes'])/int(f39[n]['svg_bytes']) for n in names}
mn = min(sz, key=sz.get); mx = max(sz, key=sz.get)
print("svg ratio min %s %.3f  max %s %.3f" % (mn, sz[mn], mx, sz[mx]))
for n in sorted(sz, key=sz.get): print("   %-14s %.2f" % (n, sz[n]))
# (4)
rm = {n: float(m39[n]['median_ms'])/float(m38[n]['median_ms']) for n in names}
rf = {n: float(f39[n]['median_ms'])/float(f38[n]['median_ms']) for n in names}
print("mermaid 39/38 geomean %.4f ; flowmaid 39/38 geomean %.4f ; ratio of geomeans speedup %.4f" % (gm(rm.values()), gm(rf.values()), gm(sp39)/gm(sp38)))
print("%-14s %-12s %7s %7s %7s  sp38 sp39" % ("name","type","merm","flow","m/f"))
for n in names:
    print("%-14s %-12s %7.3f %7.3f %7.3f  %5.0f %5.0f" % (n, f39[n]['type'], rm[n], rf[n], rm[n]/rf[n], float(m38[n]['median_ms'])/float(f38[n]['median_ms']), float(m39[n]['median_ms'])/float(f39[n]['median_ms'])))
changed = [n for n in names if f39[n]['type'] in ('flowchart','state')]
unch = [n for n in names if n not in changed]
print("changed types n=%d: mermaid %.4f flowmaid %.4f  speedup39/38 %.4f" % (len(changed), gm([rm[n] for n in changed]), gm([rf[n] for n in changed]), gm([rm[n]/rf[n] for n in changed])))
print("unchanged types n=%d (%s): mermaid %.4f flowmaid %.4f speedup39/38 %.4f" % (len(unch), ','.join(unch), gm([rm[n] for n in unch]), gm([rf[n] for n in unch]), gm([rm[n]/rf[n] for n in unch])))
dense = ['bench-75','dense-lr','dense-td']
rest_changed = [n for n in changed if n not in dense]
print("dense: mermaid %.4f flowmaid %.4f" % (gm([rm[n] for n in dense]), gm([rf[n] for n in dense])))
print("changed minus dense n=%d: mermaid %.4f flowmaid %.4f" % (len(rest_changed), gm([rm[n] for n in rest_changed]), gm([rf[n] for n in rest_changed])))
# geomeans of speedups by group
for lab, grp in (("changed", changed), ("unchanged", unch)):
    print(lab, "speedup geomean 38 %.2f -> 39 %.2f" % (gm([float(m38[n]['median_ms'])/float(f38[n]['median_ms']) for n in grp]), gm([float(m39[n]['median_ms'])/float(f39[n]['median_ms']) for n in grp])))
# min-based
rfmin = {n: float(f39[n]['min_ms'])/float(f38[n]['min_ms']) for n in names}
rmmin = {n: float(m39[n]['min_ms'])/float(m38[n]['min_ms']) for n in names}
print("min-based: mermaid %.4f flowmaid %.4f ; changed f %.4f unchanged f %.4f ; changed m %.4f unchanged m %.4f" % (gm(rmmin.values()), gm(rfmin.values()), gm([rfmin[n] for n in changed]), gm([rfmin[n] for n in unch]), gm([rmmin[n] for n in changed]), gm([rmmin[n] for n in unch])))
# (5) svg bytes
for n in dense:
    a = int(f38[n]['svg_bytes']); b = int(f39[n]['svg_bytes'])
    print("%s svg %d -> %d  (%.1f -> %.1f kB/1000; %.1f -> %.1f KiB) change %.2f%%" % (n, a, b, a/1000, b/1000, a/1024, b/1024, (b-a)/a*100))
print("svg bytes changed in flowmaid between runs:", [(n, f38[n]['svg_bytes'], f39[n]['svg_bytes']) for n in names if f38[n]['svg_bytes'] != f39[n]['svg_bytes']])
print("svg bytes changed in mermaid between runs:", [(n, m38[n]['svg_bytes'], m39[n]['svg_bytes']) for n in names if m38[n]['svg_bytes'] != m39[n]['svg_bytes']])
