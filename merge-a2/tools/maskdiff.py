import re, sys
def masks(f):
    s = open(f).read()
    out = {}
    for m in re.finditer(r'<mask id="([^"]*)"[^>]*>(.*?)</mask>', s):
        out[m.group(1)] = re.findall(r'<circle cx="([^"]*)" cy="([^"]*)" r="([^"]*)"', m.group(2))
    return out
a, b = masks(sys.argv[1]), masks(sys.argv[2])
for k in sorted(set(a) | set(b)):
    if a.get(k) != b.get(k):
        ca, cb = a.get(k, []), b.get(k, [])
        print(k, len(ca), '->', len(cb))
        print('   before', ca)
        xs = [float(c[0]) for c in cb]; ys = [float(c[1]) for c in cb]
        print('   after bbox', (min(xs), min(ys), max(xs), max(ys)) if cb else None, cb[:3], '...')
