#!/usr/bin/env bash
# pic.sh <name> <file.mmd> [mermaid.svg]: start(pre) | now(suite-on) [| mermaid]
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r7
O=$V/pic; mkdir -p $O
$V/t-pre/release/flowmaid $2 -o $O/$1.start.svg >/dev/null 2>&1
$V/t-on/release/flowmaid $2 -o $O/$1.now.svg >/dev/null 2>&1
args="start=$O/$1.start.svg now-on=$O/$1.now.svg"
[ -n "$3" ] && [ -f "$3" ] && args="$args mermaid=$3"
NODE_PATH=$(npm root -g) SCALE=${SCALE:-1} node /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/r4/png.js $O/$1.png $args
