#!/bin/bash
# ir.sh <binary> <fixture...> : instructions per scene() (200 runs)
B=$1; shift
for f in "$@"; do
  valgrind --tool=callgrind --callgrind-out-file=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b10/cg.tmp $B /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/flowcorpus/$f.mmd 50 >/dev/null 2>&1
  t=$(callgrind_annotate /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b10/cg.tmp 2>/dev/null | grep 'PROGRAM TOTALS' | awk '{gsub(",","",$1); print $1}')
  echo "$f $((t/50))"
done
