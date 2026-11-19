#!/bin/bash
# usage: runq.sh <dir-with-set> ; runs 034/035/now + toggles in parallel
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
cd $1
Q=/home/user/flowmaid/target/release/examples/quality
run() { local tag=$1; shift; env "$@" $Q set > q_$tag.tsv 2>/dev/null; }
[ -s q034.tsv ] || $S/b8b/tgt034/release/examples/quality set > q034.tsv 2>/dev/null &
[ -s q035.tsv ] || $S/b8b/tgt035/release/examples/quality set > q035.tsv 2>/dev/null &
for t in ${TAGS:-now}; do
  case $t in now) $Q set > qnow.tsv 2>/dev/null & ;; *) run $t FMX=$(echo $t | tr '+' ',') & ;; esac
  while [ $(jobs -r | wc -l) -ge 4 ]; do sleep 0.5; done
done
wait
