#!/usr/bin/env bash
# h.sh <off|on|pre>: harness (static+routed) per set -> r7/h/<mode>-<set>.tsv
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
Q=$V/r7/t-$1/release/examples/quality
O=$V/r7/h; R=/home/user/flowmaid/tests/fixtures
mkdir -p $O
for s in quality blocks notes fans; do nice $Q $R/$s > $O/$1-$s.tsv 2>/dev/null & done
wait
nice $Q $V/r5/xin > $O/$1-xin.tsv 2>/dev/null &
nice $Q $V/r5/shuf > $O/$1-shuf.tsv 2>/dev/null &
nice $Q $B/work/r8/fam > $O/$1-fam.tsv 2>/dev/null &
nice $Q $V/r6/exin > $O/$1-ex.tsv 2>/dev/null &
wait
nice $Q $V/r6/xs > $O/$1-xs.tsv 2>/dev/null &
nice $Q $V/r7/xs > $O/$1-zs.tsv 2>/dev/null &
wait
