import sys, re
def load(p):
    out = {}
    for l in open(p):
        if not (l.startswith('MUST FAIL') or l.startswith('target missed') or l.startswith('accepted')): continue
        verdict = l[:13].strip(); rest = l[14:].split()
        name, metric, val = rest[0], rest[1], rest[2]
        m = re.search(r'must (\S+) (\S+)\s+target (\S+) (\S+)\s+\[(.*?)\]', l)
        out[(verdict, name, metric)] = (val, m.groups() if m else None, l.rstrip())
    return out
a, b = load(sys.argv[1]), load(sys.argv[2])
print('old', len(a), 'new', len(b))
for k in sorted(set(a) - set(b)): print('GONE ', k, a[k][0], a[k][1])
for k in sorted(set(b) - set(a)): print('NEW  ', k, b[k][0], b[k][1])
for k in sorted(set(a) & set(b)):
    (va, ga, la), (vb, gb, lb) = a[k], b[k]
    if la == lb: continue
    what = []
    if va != vb: what.append(f'value {va}->{vb}')
    if ga and gb:
        if ga[1] != gb[1]: what.append(f'must {ga[1]}->{gb[1]}')
        if ga[3] != gb[3]: what.append(f'target {ga[3]}->{gb[3]}')
        if ga[4] != gb[4]: what.append(f'refs [{ga[4]}]->[{gb[4]}]')
    print('CHG  ', k, '; '.join(what))
