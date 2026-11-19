#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rel0402/verify-1-api
cd $V
for w in base head; do
  mkdir -p out-$w cli-$w
  ls corpus/*.mmd xcorpus/*.mmd | nice -n 15 xargs -P2 -I{} sh -c "timeout 120 ./dump-$w/target/release/dump-$w out-$w {} || echo TIMEOUT/FAIL {} >> fail-$w.txt"
  for f in corpus/*.mmd xcorpus/*.mmd; do b=$(basename $f .mmd); timeout 60 nice -n 15 ./$w/target/release/flowmaid $f > cli-$w/$b.svg 2> cli-$w/$b.err; echo "$? " >> cli-$w/$b.err; done
done
echo DONE
