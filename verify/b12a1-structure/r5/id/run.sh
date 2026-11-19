#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r5
cd $V/id
for w in pre now; do
  for p in part-aa part-ab part-ac; do
    nice $V/tp-$w/release/ident $(cat $p) > $w-$p.tsv 2> $w-$p.err &
  done
  wait
done
echo ID-DONE
