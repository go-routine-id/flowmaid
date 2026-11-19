#!/usr/bin/env bash
# h.sh <label> <quality-binary>: harness tables (static+routed) per set -> /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/merge-a1/h/<label>-<set>.tsv
L=$1; Q=$2; O=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/merge-a1/h
for s in quality blocks notes fans; do nice -n 19 $Q /home/user/flowmaid/tests/fixtures/$s > $O/$L-$s.tsv 2>/dev/null & done
wait
nice -n 19 $Q /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r5/xin > $O/$L-xin.tsv 2>/dev/null &
nice -n 19 $Q /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r5/shuf > $O/$L-shuf.tsv 2>/dev/null &
nice -n 19 $Q /home/user/flowmaid/examples > $O/$L-ex.tsv 2>/dev/null &
wait
nice -n 19 $Q /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/r9/fam > $O/$L-fam.tsv 2>/dev/null
