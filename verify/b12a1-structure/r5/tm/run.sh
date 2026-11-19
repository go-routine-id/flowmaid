#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r5
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
cd $V/tm
for r in 1 2 3; do
  for w in pre off on; do
    echo "round $r $w load $(cat /proc/loadavg) rustc $(pgrep -c rustc)" >> log.txt
    $V/t-$w/release/examples/quality $S/flowcorpus --drag > $w-$r.tsv 2>/dev/null
  done
done
echo TM-DONE >> log.txt
