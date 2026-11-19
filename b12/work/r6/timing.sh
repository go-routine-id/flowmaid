#!/usr/bin/env bash
# timing.sh <rounds> <label=quality-bin>...: interleaved static scene_ms per file (quality example), each binary per round
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
n=$1; shift
for r in $(seq 1 $n); do
  for lb in "$@"; do
    l=${lb%%=*}; q=${lb#*=}
    $q $B/work/r6/tin 2>/dev/null | awk -F'\t' -v l=$l -v r=$r 'NR>1 && $2=="static" && $1!="TOTAL"{printf "%s r%s %-20s %7.2f\n", l, r, $1, $53}'
  done
done
