#!/usr/bin/env bash
# ba.sh <binA> <binB> <out.png> <file.mmd>: static A | static B
set -euo pipefail
a=$1; b=$2; o=$3; f=$4; t=$(mktemp -d)
$a/flowmaid $f -o $t/a.svg >/dev/null 2>&1; $b/flowmaid $f -o $t/b.svg >/dev/null 2>&1
NODE_PATH=$(npm root -g) node /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/squeeze/pair.js $o $t/a.svg before $t/b.svg after ${SCALE:-1}
