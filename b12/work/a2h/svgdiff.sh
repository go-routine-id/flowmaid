#!/usr/bin/env bash
# svgdiff.sh <cliA> <cliB> <label>: static SVGs of every direction copy (a2g/dirs) with both CLIs; list the ones that differ
W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/a2h
mkdir -p $W/sd/$3/a $W/sd/$3/b
for f in $W/../a2g/dirs/*.mmd; do n=$(basename $f .mmd)
  $1 $f -o $W/sd/$3/a/$n.svg >/dev/null 2>&1; $2 $f -o $W/sd/$3/b/$n.svg >/dev/null 2>&1
  cmp -s $W/sd/$3/a/$n.svg $W/sd/$3/b/$n.svg || echo $n
done
