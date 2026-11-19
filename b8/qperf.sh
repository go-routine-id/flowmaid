#!/usr/bin/env bash
# quick perf on the 4 dense files: prints file mode route_ms scene_ms drag_ms
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
cd ${1:-/home/user/flowmaid}
cargo run --release -q --example quality -- $S/b8/q1 --drag 2>/dev/null | python3 -c "
import sys
rows=[l.rstrip('\n').split('\t') for l in sys.stdin]
h=rows[0]
idx={k:h.index(k) for k in ['route_ms','scene_ms','drag_ms','defects']}
base={'bench-75':(12.61,30.97,20.22),'dense-lr':(6.42,23.40,14.21),'dense-td':(7.36,25.34,14.24),'fanout-frames':(0.34,2.19,2.44)}
for r in rows[1:]:
    if r[0] in base:
        b=base[r[0]]
        lim=lambda x: max(1.25*x, x+1)
        print(r[0], r[1], 'route', r[idx['route_ms']], '(<=%.2f)'%lim(b[0]), 'scene', r[idx['scene_ms']], '(<=%.1f)'%(2*b[1]), 'drag', r[idx['drag_ms']], '(<=%.2f)'%lim(b[2]), 'defects', r[idx['defects']])
"
