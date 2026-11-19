import re, sys
svg = open(sys.argv[1]).read()
print(re.search(r'viewBox="([^"]*)"', svg).group(1))
paths = re.findall(r'<path d="(M [^"]*)"([^>]*)>', svg)
edges = [d for d, rest in paths if 'z' not in d and 'fill="none"' in rest]
both = 0
for d in edges:
    toks = re.findall(r'[MLC]|-?[0-9.]+', d)
    i = 0; cur = None; h = v = False
    while i < len(toks):
        t = toks[i]
        if t == 'M':
            cur = (float(toks[i+1]), float(toks[i+2])); i += 3
        elif t == 'L':
            p = (float(toks[i+1]), float(toks[i+2]))
            dx, dy = abs(p[0]-cur[0]), abs(p[1]-cur[1])
            if dy < 0.05 and dx > 2: h = True
            if dx < 0.05 and dy > 2: v = True
            cur = p; i += 3
        elif t == 'C':
            cur = (float(toks[i+5]), float(toks[i+6])); i += 7
        else:
            i += 1
    if h and v: both += 1
print('edge paths', len(edges), 'with horizontal+vertical straight runs', both)
