#!/usr/bin/env bash
# h.sh <label> <quality-binary>
L=$1; Q=$2; O=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/merge-a1/v1/h
for s in quality blocks notes fans; do nice -n 19 $Q /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/merge-a1/v1/wt/head/tests/fixtures/$s > $O/$L-$s.tsv 2>$O/$L-$s.err; done
nice -n 19 $Q /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/merge-a1/v1/wt/head/examples > $O/$L-ex.tsv 2>$O/$L-ex.err
nice -n 19 $Q /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r5/xin > $O/$L-xin.tsv 2>$O/$L-xin.err
nice -n 19 $Q /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r5/shuf > $O/$L-shuf.tsv 2>$O/$L-shuf.err
nice -n 19 $Q /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/local/in > $O/$L-fam.tsv 2>$O/$L-fam.err
echo done > $O/$L.done
