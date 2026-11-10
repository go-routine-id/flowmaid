import sys
for f in sys.argv[1:]:
    rows=[l.rstrip('\n').split('\t') for l in open(f)]
    if not rows: continue
    drop={'scene_ms','route_ms','part_ms'} if all(r[rows[0].index('scene_ms')] in ('0.00','scene_ms') for r in rows) else set()
    keep=[i for i,h in enumerate(rows[0]) if h not in drop]
    w=[max(len(r[i]) for r in rows if i<len(r)) for i in keep]
    print('==',f.split('/')[-1])
    for r in rows:
        print(' '.join(r[i].ljust(w[k]) for k,i in enumerate(keep)))
