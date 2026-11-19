# Count marker ends whose final (or first, for marker-start) segment is zero-length after print rounding.
import sys, os, re, collections
V = sys.argv[1]
pt = re.compile(r'(-?\d+(?:\.\d+)?)[ ,](-?\d+(?:\.\d+)?)')
def count(svg):
    n = 0
    for m in re.finditer(r'<path d="([^"]*)"[^>]*?(marker-(?:end|start))', svg):
        d = m.group(1)
        tag = svg[m.start():svg.find('>', m.start())]
        pts = pt.findall(d)
        if len(pts) < 2: continue
        if 'marker-end' in tag and pts[-1] == pts[-2]: n += 1
        if 'marker-start' in tag and pts[0] == pts[1]: n += 1
    return n
tot = collections.Counter(); files = collections.Counter()
lst=[l.strip() for l in open(V+'/main.lst') if l.strip()]
worse=[]; better=[]
for fn in sorted(os.listdir(V + '/dump-head')):
    i, kind, _ = fn.split('.', 2)
    b = open(V + '/dump-base/' + fn).read(); h = open(V + '/dump-head/' + fn).read()
    cb, ch = count(b), count(h)
    tot[('base', kind)] += cb; tot[('head', kind)] += ch
    if ch > cb: worse.append((lst[int(i)].split('/')[-1], kind, cb, ch))
    if ch < cb: better.append((lst[int(i)].split('/')[-1], kind, cb, ch))
kinds = sorted({k for _, k in tot})
for k in kinds: print(k, 'base', tot[('base', k)], 'head', tot[('head', k)])
print('worse', len(worse), worse[:20]); print('better', len(better), better[:20])
