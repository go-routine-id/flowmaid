#!/bin/bash
# ir.sh: callgrind instruction count per scene() for a few small corpus files (deterministic perf yardstick)
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
(cd $S/b10r2/prof && cargo build --release -q 2>&1 | grep -E '^error' -A5 | head)
for f in ${FILES:-advanced subgraph stress long-labels back-edge shapes demo wide-labels}; do
  a=$(valgrind --tool=callgrind --callgrind-out-file=/dev/null $S/b10r2/prof/target/release/prof $S/flowcorpus/$f.mmd 1 2>&1 | grep Collected | awk '{print $4}')
  b=$(valgrind --tool=callgrind --callgrind-out-file=/dev/null $S/b10r2/prof/target/release/prof $S/flowcorpus/$f.mmd 11 2>&1 | grep Collected | awk '{print $4}')
  echo "$f $(( (b - a) / 10 ))"
done
