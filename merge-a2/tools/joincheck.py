# For SVG pairs (before = no join, after = join): find the joined fill holes on each under-line and report
# any other line's gap hole, or any hop arc, lying within the joined stretch (where the under-line would be
# cut although it is the over-line, or its own hop would be erased).
import re, sys, math
def parse(f):
    s = open(f).read()
    masks = {}
    for m in re.finditer(r'<mask id="([^"]*)"[^>]*>(.*?)</mask>', s):
        masks[m.group(1)] = [tuple(map(float, c)) for c in re.findall(r'<circle cx="([^"]*)" cy="([^"]*)" r="([^"]*)"', m.group(2))]
    paths = []
    for tag in re.findall(r'<path [^>]*>', s):
        d = re.search(r' d="([^"]*)"', tag).group(1)
        mm = re.search(r'mask="url\(#([^)]*)\)"', tag)
        paths.append((d, mm.group(1) if mm else ''))
    arcs = []
    for d, mid in paths:
        for a in re.finditer(r'A ([\d.]+) [\d.]+ 0 0 [01] ([\d.-]+) ([\d.-]+)', d):
            arcs.append((float(a.group(2)), float(a.group(3)), mid))
    return masks, arcs
def run(before, after):
    mb, _ = parse(before); ma, arcs = parse(after)
    out = []
    for k, holes in ma.items():
        old = set((x, y) for x, y, r in mb.get(k, []))
        fill = [(x, y, r) for x, y, r in holes if (x, y) not in old]
        if not fill: continue
        for k2, h2 in ma.items():
            if k2 == k: continue
            for (x, y, r) in h2:
                for (fx, fy, fr) in fill:
                    if math.hypot(x - fx, y - fy) < fr:
                        out.append(f"{k}: fill hole ({fx},{fy}) covers another line's gap {k2} at ({x},{y}) -> under-line is over there")
        for (ax, ay, mid) in arcs:
            if mid != k: continue
            for (fx, fy, fr) in fill:
                if math.hypot(ax - fx, ay - fy) < fr + 8:
                    out.append(f"{k}: fill hole ({fx},{fy}) near a hop arc end ({ax},{ay}) on path masked by {mid}")
                    break
    return out
for b, a in zip(sys.argv[1::2], sys.argv[2::2]):
    o = run(b, a)
    print(a.split('/')[-1], 'ISSUES' if o else 'ok', *o[:6], sep='\n  ')
