#!/usr/bin/env bash
# near.sh <svgdir> <name...>: per label, chip box and nearest foreign stroke (side, orientation, segment)
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
D=$S/design-a2b/labels
dir=$1; shift
for n in "$@"; do
  echo "== $n"
  node $D/tools/bm-near --struct-dir $S/local/struct --detail $dir/$n.svg 2>&1 >/dev/null | awk '/^ labels:/{f=1;next} /^ [a-z]/{f=0} f' | awk 'NR==1{next} {print}' | while read -r line; do echo "  $line"; done
done
