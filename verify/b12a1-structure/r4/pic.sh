#!/usr/bin/env bash
# pic.sh <name> <file.mmd>: start | prev(649fee2, on) | now(on)
O=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r4/pic
/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r4/t-pre/release/flowmaid $2 -o $O/$1.start.svg >/dev/null 2>&1
/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r4/t-prev/release/flowmaid $2 -o $O/$1.prev.svg >/dev/null 2>&1
/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r4/t-on/release/flowmaid $2 -o $O/$1.now.svg >/dev/null 2>&1
cmp -s $O/$1.prev.svg $O/$1.now.svg && echo "$1: now == prev" || echo "$1: now differs from prev"
NODE_PATH=$(npm root -g) SCALE=${SCALE:-1} node /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/r4/png.js $O/$1.png start=$O/$1.start.svg prev=$O/$1.prev.svg now=$O/$1.now.svg
