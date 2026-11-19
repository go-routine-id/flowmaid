#!/usr/bin/env bash
# pic2.sh <name> <file.mmd>: head(suite-on) | now(suite-on) picture -> r9/pic/<name>.png
W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/r9
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
O=$W/pic; mkdir -p $O
$W/headtarget/release/flowmaid $2 -o $O/$1.head.svg >/dev/null 2>&1
${BIN:-$B/target-on/release/flowmaid} $2 -o $O/$1.now.svg >/dev/null 2>&1
NODE_PATH=$(npm root -g) SCALE=${SCALE:-1} node $B/work/r4/png.js $O/$1.png head=$O/$1.head.svg now=$O/$1.now.svg
echo $O/$1.png
