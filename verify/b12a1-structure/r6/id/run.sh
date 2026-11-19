#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r6
cd $V/id
for p in part-aa.f part-ab.f part-ac.f; do
  nice $V/tp-pre/release/ident $(cat $p) > pre-$p.tsv 2> pre-$p.err &
done
wait
for p in part-aa.f part-ab.f part-ac.f; do
  nice $V/tp-now/release/ident $(cat $p) > now-$p.tsv 2> now-$p.err &
done
wait
echo ID-DONE
