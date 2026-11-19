#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure
cd $V/r6/bulk
for p in aa ab ac ad; do
  MODE=on nice $V/r6/tp-now/release/bulkm $(cat $V/r4/bulk/part-$p) > on-$p.tsv 2> on-$p.err &
done
wait
for p in aa ab ac ad; do
  MODE=off nice $V/r6/tp-now/release/bulkm $(cat $V/r4/bulk/part-$p) > off-$p.tsv 2> off-$p.err &
done
wait
echo BULK-DONE
