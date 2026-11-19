import sys, math
base = sys.argv[1]
def tsv(p):
    rows = {}
    f = open(p); hdr = f.readline().rstrip('\n').split('\t')
    for line in f:
        c = line.rstrip('\n').split('\t')
        if c and c[0]: rows[c[0]] = dict(zip(hdr, c))
    return rows
f39 = tsv(base + '/bench039/flowmaid.tsv'); m39 = tsv(base + '/bench039/mermaid.tsv')
f38 = tsv(base + '/bench038/flowmaid.tsv'); m38 = tsv(base + '/bench038/mermaid.tsv')
def gm(xs):
    xs = list(xs); return math.exp(sum(math.log(x) for x in xs)/len(xs))
names = list(f39)
out = ['er', 'gitgraph', 'journey', 'fanout']
rest = [n for n in names if n not in out]
def stats(lab, grp):
    print("%-28s n=%2d mermaid %.3f flowmaid %.3f | speed-up geomean %.1f -> %.1f" % (lab, len(grp),
        gm(float(m39[n]['median_ms'])/float(m38[n]['median_ms']) for n in grp),
        gm(float(f39[n]['median_ms'])/float(f38[n]['median_ms']) for n in grp),
        gm(float(m38[n]['median_ms'])/float(f38[n]['median_ms']) for n in grp),
        gm(float(m39[n]['median_ms'])/float(f39[n]['median_ms']) for n in grp)))
stats("all", names)
stats("without er,gitgraph,journey,fanout", rest)
stats("only those four", out)
stats("without er,gitgraph,journey", [n for n in names if n not in out[:3]])
# what would 0.39 geomean be if those 3 had moved like the other flowmaid rows
fl = [float(f39[n]['median_ms'])/float(f38[n]['median_ms']) for n in names]
ml = [float(m39[n]['median_ms'])/float(m38[n]['median_ms']) for n in names]
print("flowmaid ratios min %.3f max %.3f ; mermaid ratios min %.3f max %.3f" % (min(fl), max(fl), min(ml), max(ml)))
print("count flowmaid faster:", sum(1 for x in fl if x < 1), "mermaid faster:", sum(1 for x in ml if x < 1))
# median of ratios
import statistics
print("median flowmaid ratio %.3f mermaid ratio %.3f" % (statistics.median(fl), statistics.median(ml)))
# arithmetic means
print("arith mean flowmaid ratio %.3f mermaid %.3f" % (sum(fl)/len(fl), sum(ml)/len(ml)))
# ratio of sums
print("sum-of-medians ratio flowmaid %.3f mermaid %.3f" % (sum(float(f39[n]['median_ms']) for n in names)/sum(float(f38[n]['median_ms']) for n in names), sum(float(m39[n]['median_ms']) for n in names)/sum(float(m38[n]['median_ms']) for n in names)))
