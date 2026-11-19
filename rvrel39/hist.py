import sys, math
base = sys.argv[1]
def tsv(p):
    rows = {}
    try:
        f = open(p)
    except OSError:
        return None
    hdr = f.readline().rstrip('\n').split('\t')
    for line in f:
        c = line.rstrip('\n').split('\t')
        if c and c[0]: rows[c[0]] = dict(zip(hdr, c))
    return rows
runs = ['bench037', 'bench038', 'bench039']
F = {r: tsv(f"{base}/{r}/flowmaid.tsv") for r in runs}
M = {r: tsv(f"{base}/{r}/mermaid.tsv") for r in runs}
names = list(F['bench039'])
print("%-14s %-12s | flowmaid median 037 038 039 | min 037 038 039 | mermaid median 037 038 039" % ("name", "type"))
for n in names:
    def g(D, r, k):
        d = D[r]
        return float(d[n][k]) if d and n in d else float('nan')
    print("%-14s %-12s | %7.3f %7.3f %7.3f | %7.3f %7.3f %7.3f | %7.1f %7.1f %7.1f" % (n, F['bench039'][n]['type'],
        g(F,'bench037','median_ms'), g(F,'bench038','median_ms'), g(F,'bench039','median_ms'),
        g(F,'bench037','min_ms'), g(F,'bench038','min_ms'), g(F,'bench039','min_ms'),
        g(M,'bench037','median_ms'), g(M,'bench038','median_ms'), g(M,'bench039','median_ms')))
def gm(xs):
    xs = list(xs); return math.exp(sum(math.log(x) for x in xs)/len(xs))
for a, b in (('bench037','bench038'), ('bench038','bench039'), ('bench037','bench039')):
    if F[a] and F[b]:
        common = [n for n in names if n in F[a] and n in M[a]]
        print(a, '->', b, 'n', len(common), 'flowmaid %.3f mermaid %.3f' % (gm(float(F[b][n]['median_ms'])/float(F[a][n]['median_ms']) for n in common), gm(float(M[b][n]['median_ms'])/float(M[a][n]['median_ms']) for n in common)),
              'speedup geomean %s %.1f %s %.1f' % (a, gm(float(M[a][n]['median_ms'])/float(F[a][n]['median_ms']) for n in common), b, gm(float(M[b][n]['median_ms'])/float(F[b][n]['median_ms']) for n in common)))
