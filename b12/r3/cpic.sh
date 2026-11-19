#!/usr/bin/env bash
# cpic.sh <out.png> <copy-name e.g. blocks.ratio4-TD> <run> [run...]: routed SVG of a direction copy from each run's quality binary
R3=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/r3
T=$R3/../tools-a2b
o=$1; n=$2; shift 2; svgs=(); tmp=$(mktemp -d); mkdir -p $tmp/in; cp $T/inputs/dirs/$n.mmd $tmp/in/ 2>/dev/null || cp $T/inputs/dirsx/$n.mmd $tmp/in/
for r in "$@"; do
  case $r in static:*) r=${r#static:}; m=static;; *) m=routed;; esac
  b=$R3/runs/$r/bin; [ -d $b ] || b=$T/$r/bin
  mkdir -p $tmp/$r; FMDBG_COR= $b/quality $tmp/in --svg $tmp/$r > /dev/null 2>&1; cp $tmp/$r/$n.$m.svg $tmp/$r-$m.svg; svgs+=($tmp/$r-$m.svg)
done
NODE_PATH=$(npm root -g) node $R3/../sqf/tools/png.js $o "${svgs[@]}"; rm -rf $tmp
