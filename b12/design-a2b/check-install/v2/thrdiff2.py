import json, sys
a = json.load(open(sys.argv[1])); b = json.load(open(sys.argv[2]))
print('note same' if a.get('note') == b.get('note') else 'NOTE DIFFERS')
ra, rb = a['rules'], b['rules']
print(len(ra), len(rb))
same = 0
for i, (x, y) in enumerate(zip(ra, rb)):
    if x == y: same += 1; continue
    keys = sorted(set(x) | set(y))
    d = {k: (x.get(k), y.get(k)) for k in keys if x.get(k) != y.get(k)}
    # direction
    op = y.get('op', '<=')
    verdict = []
    for side in ('must', 'target'):
        if side in d:
            o, n = d[side]
            if isinstance(o, (int, float)) and isinstance(n, (int, float)):
                t = 'tighter' if (n < o if op == '<=' else n > o) else ('looser' if n != o else 'same')
            else: t = 'NONNUMERIC'
            verdict.append(f'{side}:{t}')
    other = [k for k in d if k not in ('must', 'target', 'why')]
    print(i, x['metric'], op, x['scope'], {k: v for k, v in d.items() if k != 'why'}, verdict, 'other-fields:' + str(other) if other else '')
    if 'why' in d:
        print('   why old:', d['why'][0]); print('   why new:', d['why'][1])
print('unchanged', same)
