import sys, csv
def load(p):
    with open(p) as f:
        rows = [l.rstrip('\n').split('\t') for l in f]
    return rows[0], rows[1:]
for t in sys.argv[3:]:
    ho, ro = load(sys.argv[1]+'/'+t); hn, rn = load(sys.argv[2]+'/'+t)
    print('==', t, 'hdr same' if ho==hn else 'HDR DIFF', len(ro), len(rn))
    if ho != hn:
        print('  old-only', set(ho)-set(hn), 'new-only', set(hn)-set(ho))
    keyo = [(r[0], r[1], r[2]) for r in ro]; keyn = [(r[0], r[1], r[2]) for r in rn]
    if keyo != keyn: print('  KEY DIFF')
    cnt = {}
    for a, b in zip(ro, rn):
        da = dict(zip(ho, a)); db = dict(zip(hn, b))
        for c in hn:
            if da.get(c) != db.get(c):
                cnt[c] = cnt.get(c, 0) + 1
    print('  changed columns:', cnt)
