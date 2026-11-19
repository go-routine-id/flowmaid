#!/usr/bin/env bash
cd /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/r9/bulk
for p in aa ab ac ad; do
  MODE=on nice ./bulkm-p3 $(cat /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r4/bulk/part-$p) > on-$p.tsv 2> on-$p.err &
done
wait
echo ON-DONE
