#!/usr/bin/env bash
# cmp.sh <labelA> <labelB> [sets...]: qdiff per set, summary lines
F=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12a2b/fr4
Q=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a2b-base/fr4/tools/qdiff.py
a=$1; b=$2; shift 2; sets=${*:-repro tw rnd sw sw72 vin}
for s in $sets; do echo "## $s $a -> $b"; python3 $Q $F/s-$a/$s/q.tsv $F/s-$b/$s/q.tsv | grep -v TOTAL; done
