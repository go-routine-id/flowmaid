#!/usr/bin/env bash
# Compare base vs current flowmaid output over examples, corpus, tests/fixtures and par inputs.
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
B=$S/par/basewt/target/release/flowmaid; N=$S/wt/par/target/release/flowmaid
(cd $S/wt/par && cargo build --release -q --bin flowmaid)
same=0; diff=0
for f in $S/wt/par/examples/*.mmd $S/cmp/corpus/*.mmd $(find $S/wt/par/tests/fixtures -name '*.mmd') $S/par/in/*.mmd $S/par/sweep/in/*.mmd "$@"; do
  a=$($B "$f" 2>&1 | md5sum); b=$($N "$f" 2>&1 | md5sum)
  if [ "$a" == "$b" ]; then same=$((same+1)); else diff=$((diff+1)); echo "DIFF ${f#$S/}"; fi
done
echo "same=$same diff=$diff"
