import math, csv, sys
base = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/compare/'
def load(p):
    with open(p) as f:
        r = csv.DictReader(f, delimiter='\t')
        return {row['name']: row for row in r}
def gm(xs):
    return math.exp(sum(math.log(x) for x in xs)/len(xs))
f39 = load(base+'bench039/flowmaid.tsv'); m39 = load(base+'bench039/mermaid.tsv')
f38 = load(base+'bench038/flowmaid.tsv'); m38 = load(base+'bench038/mermaid.tsv')
names = list(f39)
print(len(names))
r39 = [float(m39[n]['median_ms'])/float(f39[n]['median_ms']) for n in names]
r38 = [float(m38[n]['median_ms'])/float(f38[n]['median_ms']) for n in names]
print('gm39', gm(r39), 'gm38', gm(r38))
print('under 1ms 39', sum(1 for n in names if float(f39[n]['median_ms'])<1.0), '38', sum(1 for n in names if float(f38[n]['median_ms'])<1.0))
sz = [int(m39[n]['svg_bytes'])/int(f39[n]['svg_bytes']) for n in names]
print('svg ratio min max', min(sz), max(sz))
for n in names: print(n, round(int(m39[n]['svg_bytes'])/int(f39[n]['svg_bytes']),2))
mr = [float(m39[n]['median_ms'])/float(m38[n]['median_ms']) for n in names]
fr = [float(f39[n]['median_ms'])/float(f38[n]['median_ms']) for n in names]
print('mermaid gm ratio', gm(mr), 'flowmaid gm ratio', gm(fr))
print('mermaid sum ratio', sum(float(m39[n]['median_ms']) for n in names)/sum(float(m38[n]['median_ms']) for n in names))
print('flowmaid sum ratio', sum(float(f39[n]['median_ms']) for n in names)/sum(float(f38[n]['median_ms']) for n in names))
import statistics
print('mermaid median ratio', statistics.median(mr), 'flowmaid median ratio', statistics.median(fr))
fs = [n for n in names if f39[n]['type'] in ('flowchart','state')]
ot = [n for n in names if n not in fs]
print(len(fs), len(ot))
print('flowmaid fs', gm([float(f39[n]['median_ms'])/float(f38[n]['median_ms']) for n in fs]), 'other', gm([float(f39[n]['median_ms'])/float(f38[n]['median_ms']) for n in ot]))
print('mermaid fs', gm([float(m39[n]['median_ms'])/float(m38[n]['median_ms']) for n in fs]), 'other', gm([float(m39[n]['median_ms'])/float(m38[n]['median_ms']) for n in ot]))
for n in names:
    print(f"{n:14s} f {float(f39[n]['median_ms'])/float(f38[n]['median_ms']):.2f} m {float(m39[n]['median_ms'])/float(m38[n]['median_ms']):.2f} svg38 {f38[n]['svg_bytes']} svg39 {f39[n]['svg_bytes']} {int(f39[n]['svg_bytes'])/int(f38[n]['svg_bytes']):.3f}")
