#!/usr/bin/env bash
# h.sh <label>: suite-on harness (static+routed rows) on repository fixture dirs + stress inputs -> a2b/h/<label>-<set>.tsv
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r5
Q=${Q:-$B/target-on/release/examples/quality}
O=$B/work/a2b/h; mkdir -p $O
R=/home/user/flowmaid/tests/fixtures
for s in quality blocks notes fans; do $Q $R/$s > $O/$1-$s.tsv 2>/dev/null & done
$Q $V/xin > $O/$1-xin.tsv 2>/dev/null &
wait
