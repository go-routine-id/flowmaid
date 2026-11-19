#!/usr/bin/env bash
# pic.sh <name> <file.mmd> [bin]: start | on(now) picture -> $B/work/r7/pic/<name>.png
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
O=$B/work/r7/pic; mkdir -p $O
BIN=${3:-$B/target-on/release/flowmaid}
$B/start/flowmaid $2 -o $O/$1.start.svg >/dev/null 2>&1
$BIN $2 -o $O/$1.on.svg >/dev/null 2>&1
NODE_PATH=$(npm root -g) node $B/work/r4/png.js $O/$1.png start=$O/$1.start.svg on=$O/$1.on.svg
echo $O/$1.png
