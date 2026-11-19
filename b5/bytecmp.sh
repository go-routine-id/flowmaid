#!/usr/bin/env bash
# Compare default-theme output of the HEAD-baseline binary and the working tree binary on every .mmd we have.
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
B=$S/b5/base/target/release/flowmaid; N=/home/user/flowmaid/target/release/flowmaid
n=0; d=0
for f in $(find /home/user/flowmaid/examples /home/user/flowmaid/tests $S/cmp/corpus $S/flowcorpus -name '*.mmd' 2>/dev/null | sort); do
  n=$((n+1))
  a=$($B "$f" 2>&1 | md5sum); b=$($N "$f" 2>&1 | md5sum)
  if [ "$a" != "$b" ]; then d=$((d+1)); echo "DIFF $f"; fi
done
echo "compared $n files, $d differ"
