#!/usr/bin/env bash
# clscmp.sh <out>: compare class/er inputs (non-flowchart) between r3pre, pre and now
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
PRE=$S/b7r1/pre/target/release/flowmaid
R3=$S/b7fix/r3pre/target/release/flowmaid
NOW=/home/user/flowmaid/target/release/flowmaid
O=$1; : > $O
{ cat $S/verify/a1-class/r6/final/all_inputs.txt; ls $S/verify/a1-class/r6/final/uniq/*.mmd; ls /home/user/flowmaid/examples/*.mmd; find /home/user/flowmaid/tests/fixtures -name '*.mmd'; } | sort -u | while read f; do
  [ -f "$f" ] || continue
  hdr=$(grep -v '^\s*%%' "$f" | grep -v '^\s*$' | grep -v '^---' | grep -vi '^\s*\(title\|config\|theme\|look\|layout\)\s*:' | head -1 | awk '{print $1}')
  case "$hdr" in flowchart*|graph*|stateDiagram*) continue;; esac
  a=$($PRE "$f" 2>/dev/null | md5sum | cut -c1-12)
  b=$($R3 "$f" 2>/dev/null | md5sum | cut -c1-12)
  c=$($NOW "$f" 2>/dev/null | md5sum | cut -c1-12)
  echo "$hdr $a $b $c $f" >> $O
done
