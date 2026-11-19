#!/usr/bin/env bash
# pair.sh <out.png> <mode static|routed> <input.mmd> <bin1> <bin2> [bin3]: side-by-side renders via quality --svg
set -uo pipefail
out=$1; mode=$2; inp=$3; shift 3
T=$(mktemp -d); n=$(basename $inp .mmd); mkdir -p $T/in; cp $inp $T/in/
svgs=()
for b in "$@"; do l=$(basename $(dirname $b))-$(basename $b); mkdir -p $T/$l; $b/quality $T/in --svg $T/$l >/dev/null 2>&1; cp $T/$l/$n.$mode.svg $T/$l.svg 2>/dev/null; svgs+=($T/$l.svg); done
NODE_PATH=$(npm root -g) node /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/sqf/tools/png.js $out "${svgs[@]}"
rm -rf $T
