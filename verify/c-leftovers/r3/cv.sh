#!/usr/bin/env bash
# cv.sh [-m] <file.mmd> <tag>... : class drawing pre | now (| mermaid) side by side -> r3/vis/<name>.<tag>.png
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad; V=$S/verify/c-leftovers
MM=0; if [ "$1" = "-m" ]; then MM=1; shift; fi
f=$(readlink -f $1); shift; n=$(basename $f .mmd); mkdir -p $V/r3/vis $V/r3/vis/mm
for t in "$@"; do
  $V/hp/target/release/clsvg $f $t $V/r3/vis/$n.$t.pre.svg 2>/dev/null
  $V/hn/target/release/clsvg $f $t $V/r3/vis/$n.$t.now.svg 2>$V/r3/vis/$n.$t.err
  items="pre=$V/r3/vis/$n.$t.pre.svg now=$V/r3/vis/$n.$t.now.svg"
  if [ $MM = 1 ]; then (cd $V/r3/vis/mm && NODE_PATH=$(npm root -g) node $S/tools/mmrender.js $V/r3/vis/mm $(readlink -f $f) >/dev/null 2>&1); items="mermaid=$V/r3/vis/mm/$n.mermaid.png $items"; fi
  NODE_PATH=$(npm root -g) SCALE=${SCALE:-1} node $V/tools/side.js $V/r3/vis/$n.$t.png $items
  echo "$V/r3/vis/$n.$t.png $(cat $V/r3/vis/$n.$t.err)"
done
