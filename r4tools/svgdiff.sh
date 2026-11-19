#!/usr/bin/env bash
# usage: svgdiff.sh <binA> <binB> -> lists inputs whose SVG differ
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
A=$1; B=$2
files=$(ls $S/cmp/corpus/*.mmd /home/user/flowmaid/examples/*.mmd $(find /home/user/flowmaid/tests/fixtures -name '*.mmd') $S/verify/fu/v2/dump/*.mmd $S/verify/b7-labels-ports/in/*.mmd $S/verify/b7-labels-ports/r3/in/*.mmd $S/b7r3/in/*.mmd 2>/dev/null)
n=0; d=0
for f in $files; do n=$((n+1)); a=$($A $f 2>/dev/null | md5sum); b=$($B $f 2>/dev/null | md5sum); if [ "$a" != "$b" ]; then d=$((d+1)); echo "DIFF $f"; fi; done
echo "$d of $n differ"
