#!/usr/bin/env bash
# hh.sh <label> <quality-binary>: suite-on harness over every set -> r9/h/<label>-<set>.tsv
W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/r9
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure
Q=$2; O=$W/h; mkdir -p $O; R=/home/user/flowmaid/tests/fixtures
for s in quality blocks notes fans; do $Q $R/$s > $O/$1-$s.tsv 2>/dev/null & done
wait
$Q $V/r5/xin > $O/$1-xin.tsv 2>/dev/null &
$Q $V/r5/shuf > $O/$1-shuf.tsv 2>/dev/null &
$Q $V/r6/xs > $O/$1-xs.tsv 2>/dev/null &
$Q $V/r6/exin > $O/$1-ex.tsv 2>/dev/null &
wait
$Q $W/../r8/fam > $O/$1-fam.tsv 2>/dev/null
$Q $W/f23 > $O/$1-f23.tsv 2>/dev/null
