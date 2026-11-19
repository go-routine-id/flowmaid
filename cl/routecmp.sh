#!/usr/bin/env bash
# routecmp.sh <listfile> : inputs whose static / routed / dragged SVG differs between probe-base and probe (dragsvg)
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
A=$S/cl/probe-base/target/release/dragsvg; B=$S/cl/probe/target/release/dragsvg; n=0; d=0
while read -r f; do
  grep -qE "^\s*(flowchart|graph|stateDiagram)" "$f" || continue
  n=$((n+1))
  for mode in "scene" "route" "pidx 0 40 25" "pidx 50 -60 35" "pidx 99 25 -45"; do
    if ! cmp -s <(timeout 60 $A "$f" $mode 2>&1 | grep -v "^thread\|^note:") <(timeout 60 $B "$f" $mode 2>&1 | grep -v "^thread\|^note:"); then d=$((d+1)); echo "DIFF $mode $f"; fi
  done
done < "$1"
echo "compared $n, differ $d"
