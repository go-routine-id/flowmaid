#!/usr/bin/env bash
# vis.sh [-m] <file.mmd>... : pre | now (| mermaid with -m) side by side -> vis/<name>.png
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad; V=$S/verify/c-leftovers
MM=0; if [ "$1" = "-m" ]; then MM=1; shift; fi
mkdir -p $V/r2/vis/pre $V/r2/vis/now $V/r2/vis/mm
for f in "$@"; do n=$(basename $f .mmd)
  $V/pre/target/release/flowmaid $f > $V/r2/vis/pre/$n.svg 2>$V/r2/vis/pre/$n.err || echo "pre ERR $n: $(head -c 200 $V/r2/vis/pre/$n.err)"
  $S/wt/par/target/release/flowmaid $f > $V/r2/vis/now/$n.svg 2>$V/r2/vis/now/$n.err || echo "now ERR $n: $(head -c 200 $V/r2/vis/now/$n.err)"
  items="pre=$V/r2/vis/pre/$n.svg now=$V/r2/vis/now/$n.svg"
  if [ $MM = 1 ]; then (cd $V/r2/vis/mm && NODE_PATH=$(npm root -g) node $S/tools/mmrender.js $V/r2/vis/mm $(readlink -f $f) >/dev/null 2>&1); items="mermaid=$V/r2/vis/mm/$n.mermaid.png $items"; fi
  NODE_PATH=$(npm root -g) node $V/tools/side.js $V/r2/vis/$n.png $items
done
