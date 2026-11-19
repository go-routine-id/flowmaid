#!/usr/bin/env bash
for r in 1 2 3; do for b in on-head def-head on-base def-base; do
  /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/r3b/perfrun.sh /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/r3c/bins/$b b12a3fr2-$b-$r
done; done
echo PERF-DONE
