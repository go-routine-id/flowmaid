#!/bin/bash
# usage: FMX=.. rob.sh -> prints width stats per family
cd /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b8b/r1/rob
for f in *.mmd; do w=$(/home/user/flowmaid/target/release/flowmaid $f 2>/dev/null | grep -o 'viewBox="[^"]*"' | head -1 | awk '{print $3}'); echo "$f $w"; done | python3 -c "
import sys,collections
d=collections.defaultdict(list)
for l in sys.stdin:
    f,w=l.split(); fam=f.rsplit('_',1)[0]; d[fam].append((f,float(w)))
for fam,v in sorted(d.items()):
    ws=[w for _,w in v]; base=[w for f,w in v if f.endswith('_0.mmd') or f.endswith('_src.mmd')]
    b=base[0] if base else ws[0]
    print(f'{fam:16s} base {b:.0f} min {min(ws):.0f} max {max(ws):.0f} spread {max(abs(w-b) for w in ws)/b*100:.1f}% n={len(ws)}')
"
