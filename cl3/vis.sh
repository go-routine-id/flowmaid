#!/usr/bin/env bash
# vis.sh [-m] <mode> <file.mmd>... : (mermaid |) pre | r1 | now side by side -> /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/cl3/vis/<name>.<mode>.png
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad; V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/c-leftovers; D=$S/cl3/vis
MM=0; if [ "$1" = "-m" ]; then MM=1; shift; fi
mode=$1; shift
mkdir -p $D/pre $D/r1 $D/now $D/mm
for f in "$@"; do n=$(basename $f .mmd)
  $V/hp/target/release/clsvg $f $mode $D/pre/$n.svg 2>/dev/null
  $V/hn/target/release/clsvg $f $mode $D/r1/$n.svg 2>/dev/null
  $S/cl3/hn/target/release/clsvg $f $mode $D/now/$n.svg 2>/dev/null
  items="pre=$D/pre/$n.svg r1=$D/r1/$n.svg now=$D/now/$n.svg"
  if [ $MM = 1 ]; then (cd $D/mm && NODE_PATH=$(npm root -g) node $S/tools/mmrender.js $D/mm $(readlink -f $f) >/dev/null 2>&1); items="mermaid=$D/mm/$n.mermaid.png $items"; fi
  NODE_PATH=$(npm root -g) node $V/tools/side.js $D/$n.$mode.png $items
done
