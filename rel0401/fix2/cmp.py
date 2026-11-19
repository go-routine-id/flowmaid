import sys, collections
base, head = sys.argv[1], sys.argv[2]
def load(p):
    d = {}
    for l in open(p):
        f = l.rstrip('\n').split('\t')
        if len(f) < 3: continue
        d[(f[0], f[1])] = (f[2], f[3] if len(f) > 3 else '')
    return d
B, H = load(base), load(head)
def info(s):
    out = {}
    for kv in s.split(';'):
        if '=' in kv:
            k, v = kv.split('=', 1); out[k] = v
    return out
cnt = collections.Counter(); bad = []
for k, (hb, ib) in B.items():
    if k not in H: cnt['missing'] += 1; continue
    hh, ih = H[k]
    kind = k[1]
    I = info(ih)
    same = hb == hh
    if kind in ('render', 'scene'):
        cnt[('static', same)] += 1
        if not same: bad.append(('static-changed',) + k)
        continue
    vb = I.get('vb')
    if vb is None:
        cnt[(kind[:2], 'novb', same)] += 1
        if not same: bad.append(('novb-changed',) + k)
        continue
    x, y = map(float, vb.split(','))
    if x == 0 and y == 0:
        cnt[('routed vb00', same)] += 1
        if not same: bad.append(('vb00-changed',) + k)
    elif x > 0 or y > 0:
        cnt[('routed vb positive', same)] += 1
        bad.append(('vb-positive', ) + k + (vb,))
    else:
        cnt[('routed overflow', same)] += 1
        o = I.get('origin')
        if 'vbok' in I and I['vbok'] != 'true': bad.append(('vbok-false',) + k)
    if I.get('nodes_ok') == 'false': bad.append(('nodes-moved',) + k)
    if I.get('pickbad', '0') != '0': cnt['pickbad'] += 1
    if I.get('canvas') == 'false': bad.append(('canvas-false',) + k)
for k, v in sorted(cnt.items(), key=str): print(k, v)
print('bad', len(bad))
for b in bad[:40]: print(*b)
# try rows
t = collections.Counter(v[1] for k, v in H.items() if k[1] == 'try')
print('try', dict(t))
