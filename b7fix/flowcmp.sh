#!/usr/bin/env bash
# flowcmp.sh <ref-bin> <out>: list flowchart/state inputs whose SVG differs between ref-bin and the repo build
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
REF=$1; O=$2; NOW=/home/user/flowmaid/target/release/flowmaid; : > $O
for f in $S/cmp/corpus/*.mmd /home/user/flowmaid/examples/*.mmd $(find /home/user/flowmaid/tests/fixtures -name '*.mmd') $S/verify/fu/v2/dump/*.mmd $S/verify/b3-backedges/r6/allflow/*.mmd $S/b7r3/qin/*.mmd $S/verify/b7-labels-ports/r4/in/*.mmd $S/verify/b7-labels-ports/r3/in/*.mmd $S/b7fix/in/*.mmd; do
  [ -f "$f" ] || continue
  a=$($REF "$f" 2>/dev/null | md5sum | cut -c1-12); c=$($NOW "$f" 2>/dev/null | md5sum | cut -c1-12)
  [ "$a" != "$c" ] && echo "$f" >> $O
done
echo "$(wc -l < $O) differ"
