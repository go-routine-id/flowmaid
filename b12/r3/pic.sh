#!/usr/bin/env bash
# pic.sh <out.png> <name> <run1> [run2 ...]: routed SVG of local input <name> from each run (r3/runs/<run>/local/q), side by side;
# a run "static:<run>" takes the static twin.
R3=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/r3
o=$1; n=$2; shift 2; svgs=()
for r in "$@"; do
  case $r in static:*) r=${r#static:}; m=static;; *) m=routed;; esac
  d=$R3/runs/$r/local/q; [ -d $d ] || d=$R3/../tools-a2b/$r/local/q
  svgs+=($d/$n.$m.svg)
done
NODE_PATH=$(npm root -g) node $R3/../sqf/tools/png.js $o "${svgs[@]}"
