#!/bin/bash
# resync.sh: copy the repo's sources into fmprof except edgepass.rs/lib.rs (timers), re-apply inline(never) markers
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b10r3
for f in /home/user/flowmaid/src/*.rs; do n=$(basename $f); [ $n = edgepass.rs ] || [ $n = lib.rs ] || cp $f $B/fmprof/src/; done
python3 - <<'PY'
import re
B='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b10r3/fmprof/src/'
p=B+'router.rs'; s=open(p).read()
for fn in ['fn spread(','fn place_group(','fn parallel_pieces(','fn merge_jog(','fn info_of(','fn reorder(','fn straight_between_ports(','fn split_channel(','fn shifted(','fn mark_crossed_labels(','fn drawn_with(','fn slide_range(','fn close_moves(']:
    s=s.replace('\n'+fn,'\n#[inline(never)]\n'+fn,1)
    s=s.replace('\npub(crate) '+fn,'\n#[inline(never)]\npub(crate) '+fn,1)
open(p,'w').write(s)
p=B+'scene.rs'; s=open(p).read()
s=re.sub(r'\n(pub(?:\(crate\))? fn edge_polyline\()',r'\n#[inline(never)]\n\1',s,1)
open(p,'w').write(s)
PY
