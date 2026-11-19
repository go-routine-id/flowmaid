#!/usr/bin/env bash
# bytes.sh: render every input with pre, r3pre and now CLIs; report diffs.
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
V=$S/verify/b7-labels-ports
O=$V/r4/bytes
PRE=$V/pre/target/release/flowmaid
R3=$V/r3pre/target/release/flowmaid
NOW=/home/user/flowmaid/target/release/flowmaid
: > $O/list.txt
for f in $S/cmp/corpus/*.mmd /home/user/flowmaid/examples/*.mmd $(find /home/user/flowmaid/tests/fixtures -name '*.mmd') $S/verify/fu/v2/dump/*.mmd $S/verify/b3-backedges/r6/allflow/*.mmd $S/verify/a1-class/r6/final/uniq/*.mmd $S/b7r3/qin/*.mmd $S/verify/a8-architecture/fz/*.mmd $S/verify/a7-sankey-mindmap/r3/ring/*.mmd $V/r4/in/*.mmd $V/r3/in/*.mmd; do
  [ -f "$f" ] || continue
  hdr=$(grep -v '^\s*%%' "$f" | grep -v '^\s*$' | grep -v '^---' | grep -vi '^\s*\(title\|config\|theme\|look\|layout\)\s*:' | head -1 | awk '{print $1}')
  a=$($PRE "$f" 2>/dev/null | md5sum | cut -c1-12)
  b=$($R3 "$f" 2>/dev/null | md5sum | cut -c1-12)
  c=$($NOW "$f" 2>/dev/null | md5sum | cut -c1-12)
  echo "$hdr $a $b $c $f" >> $O/list.txt
done
