import sys, collections
def load(p):
    d = {}
    for l in open(p):
        f, k, h, info = l.rstrip('\n').split('\t', 3)
        d[(f, k)] = (h, info)
    return d
b = load(sys.argv[1]); h = load(sys.argv[2])
def kv(info):
    return dict(x.split('=', 1) for x in info.split(';') if '=' in x)
cat = collections.Counter(); ex = collections.defaultdict(list)
for key in sorted(set(b) | set(h)):
    f, k = key
    if k == 'try':
        i = h.get(key, ('', ''))[1]
        if i != 'Ok("")': cat['try_bad'] += 1; ex['try_bad'].append((f, i))
        continue
    if key not in b: cat['only_head:'+k] += 1; ex['only_head'].append(key); continue
    if key not in h: cat['only_base:'+k] += 1; ex['only_base'].append(key); continue
    bh, bi = b[key]; hh, hi = h[key]
    hk = kv(hi)
    for flag in ('nodes_ok', 'vbok', 'canvas', 'static0'):
        if hk.get(flag) == 'false': cat['bad_'+flag+':'+k.split('_')[0]] += 1; ex['bad_'+flag].append((f, k, hi))
    if hk.get('cardmiss', '0') != '0': cat['cardmiss:'+k] += 1; ex['cardmiss'].append((f, k, hi))
    if hk.get('pickbad', '0') != kv(bi).get('pickbad', '0'): cat['pick_diff'] += 1; ex['pick_diff'].append((f, k, bi, hi))
    if 'PANIC' in hi or 'PANIC' in bi: cat['panic'] += 1; ex['panic'].append((f, k, bi, hi))
    if bh == hh:
        cat['same:'+k] += 1
        continue
    # classify by head's viewBox origin
    vb = hk.get('vb', 'nan,nan').split(',')
    try: vx, vy = float(vb[0]), float(vb[1])
    except: vx = vy = float('nan')
    o = hk.get('origin', '')
    if k == 'render':
        c = 'render:' + ('now_err' if hh == '-' else 'changed')
    elif vx < 0 or vy < 0: c = 'diff_overflow:' + k
    elif vx > 0 or vy > 0: c = 'diff_inward:' + k
    else: c = 'diff_vb00:' + k
    cat[c] += 1; ex[c].append((f, k, bi, hi))
for c in sorted(cat): print(f'{cat[c]:6d} {c}')
import json
json.dump({k: v for k, v in ex.items()}, open(sys.argv[3], 'w'), indent=0)
