import json, sys
CH = {'bends','bends_max','bends_max_edge','straight_max','straight_max_edge','lane_gap_min','lanes_max'}
def walk(b, metric, out):
    if b is None or isinstance(b, (int, float)): return
    if isinstance(b, dict):
        if 'ref' in b: out.append((b['ref'], b.get('metric', metric)))
        for k in ('max', 'min'):
            if k in b:
                for x in b[k]: walk(x, metric, out)
for fn in sys.argv[1:]:
    rules = json.load(open(fn))['rules']
    reads = []; metrics = set()
    for i, r in enumerate(rules):
        metrics.add(r['metric'])
        out = []
        walk(r.get('must'), r['metric'], out); walk(r.get('target'), r['metric'], out)
        for t, m in out: reads.append((i, r['metric'], t, m))
        for k in ('must','target'):
            b = r.get(k)
            if isinstance(b, dict) and not ({'ref','max','min','same_as','within','of','factor','add','metric'} >= set(b)):
                print('UNKNOWN bound keys', fn, i, b)
    print('==', fn, len(rules), 'rules; metrics judged:', sorted(metrics))
    print('  (table, column) read by refs:', sorted(set((t, m) for _, _, t, m in reads)))
    bad = [x for x in reads if x[3] in CH]
    print('  refs that read a changed column:', bad)
    print('  rules ON a changed metric:', [(i, r['metric'], r['scope'] if not isinstance(r['scope'], list) else r['scope'][:3], r.get('must'), r.get('target')) for i, r in enumerate(rules) if r['metric'] in CH])
