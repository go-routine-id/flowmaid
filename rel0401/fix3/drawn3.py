import sys, collections
def load(p):
    rows = {}
    for l in open(p):
        f = l.rstrip('\n').split('\t')
        if len(f) < 4: continue
        info = dict(kv.split('=', 1) for kv in f[3].split(';') if '=' in kv)
        rows[(f[0], f[1])] = (f[2], info)
    return rows
for tag in sys.argv[1:]:
    R = load(tag)
    c = collections.Counter(); ex = []
    for (fn, kind), (h, I) in R.items():
        if 'dchk' not in I or kind in ('scene', 'render'): continue
        x, y = map(float, I['vb'].split(','))
        over = not (x == 0 and y == 0)
        key = 'vb!=0' if over else 'vb=0'
        c[(key, 'svgs')] += 1
        c[(key, 'nodes checked')] += int(I['dchk'])
        if int(I['doff']) > 0:
            c[(key, 'svgs with a node drawn off its scene centre')] += 1
            if over and len(ex) < 5: ex.append((fn.split('/')[-1], kind, I['vb'], I['doff']))
        if int(I['dpick']) > 0:
            c[(key, 'svgs with a pick at a drawn centre missing its node')] += 1
        if I.get('nodes_ok') == 'false': c[(key, 'NODES MOVED')] += 1
        if I.get('vbok') == 'false': c[(key, 'vb != origin on overflowing axis')] += 1
        if I.get('pickbad', '0') != '0': c[(key, 'pick at scene centre fails')] += 1
    print('==', tag)
    for k, v in sorted(c.items()): print('  ', v, k)
    for e in ex: print('   e.g.', e)
