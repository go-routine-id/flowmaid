#!/usr/bin/env bash
# route() repair work spent (deterministic proxy for drag cost); usage: work.sh <binary> files...
B=$1; shift
for f in "$@"; do
  FMDBG_REPAIR=1 $B $f 1 2>&1 | awk -v n=$(basename $f .mmd) '/REPAIR mode=false/{on=1} /REPAIR mode=true/{on=0} on && /EDGE/{s+=$4; c++} END {printf "%s route_work=%d searches=%d\n", n, s, c}'
done
