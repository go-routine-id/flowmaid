import re, sys
def paths(f):
    s = open(f).read()
    return re.findall(r'<path d="([^"]*)"', s), s
def nums(d):
    return [float(x) for x in re.findall(r'-?\d+(?:\.\d+)?', d)]
a, sa = paths(sys.argv[1]); b, sb = paths(sys.argv[2])
print(f"paths {len(a)} vs {len(b)}; masks {sa.count('<mask ')} vs {sb.count('<mask ')}; circles {sa.count('<circle')} vs {sb.count('<circle')}")
for i, (x, y) in enumerate(zip(a, b)):
    if x != y:
        nx, ny = nums(x), nums(y)
        diffs = [(k, nx[k], ny[k]) for k in range(min(len(nx), len(ny))) if nx[k] != ny[k]]
        print(f"path {i}: len {len(nx)} vs {len(ny)}; start {nx[:2]} end {nx[-2:]} -> start {ny[:2]} end {ny[-2:]}; first diffs {diffs[:6]}")
