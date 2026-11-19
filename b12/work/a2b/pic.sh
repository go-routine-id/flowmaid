#!/usr/bin/env bash
# pic.sh <name> <file.mmd> [bin...]: pre(b12a2 start, blocks) | now (target-on) -> a2b/pic/<name>.png
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12; W=$B/work/a2b
O=$W/pic; mkdir -p $O
BIN=${BIN:-$B/target-on/release/flowmaid}
items=()
$B/work/a2/flowmaid-pre-on $2 -o $O/$1.pre.svg >/dev/null 2>&1; items+=(pre=$O/$1.pre.svg)
$BIN $2 -o $O/$1.now.svg >/dev/null 2>&1; items+=(now=$O/$1.now.svg)
NODE_PATH=$(npm root -g) SCALE=${SCALE:-1} node $B/work/r4/png.js $O/$1.png "${items[@]}"
echo $O/$1.png
