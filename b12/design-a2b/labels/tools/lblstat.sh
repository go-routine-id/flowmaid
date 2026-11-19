#!/usr/bin/env bash
# lblstat.sh <name> <dir...>: per label len/off/endpx/gap per output dir
D=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/labels
n=$1; shift
for d in "$@"; do
  node $D/tools/bm-near --struct-dir $D/judge/struct --detail $d/$n.svg 2>&1 >/dev/null | awk '/^ labels:/{f=1;next} /^ [a-z]/{f=0} f' | awk -v tag=$(basename $d) 'NR>1 {n=NF; printf "%-6s %-14s L %7.1f off %5.3f endpx %6.1f gap %6.1f\n", tag, $1, $(n-12), $(n-10), $(n-9), $(n-5)}'
done
