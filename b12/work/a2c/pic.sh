#!/usr/bin/env bash
# pic.sh <name> <file.mmd> [label=bin ...]: renders side by side -> a2c/pic/<name>.png (default: pre | now)
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12; W=$B/work/a2c
O=$W/pic; mkdir -p $O
name=$1; f=$2; shift 2
[ $# = 0 ] && set -- pre=$B/work/a2/flowmaid-pre-on now=$B/target-on/release/flowmaid
items=()
for kv in "$@"; do l=${kv%%=*}; bin=${kv#*=}; $bin $f -o $O/$name.$l.svg >/dev/null 2>&1; items+=($l=$O/$name.$l.svg); done
NODE_PATH=$(npm root -g) SCALE=${SCALE:-1} node $B/work/r4/png.js $O/$name.png "${items[@]}" >/dev/null 2>&1
echo $O/$name.png
