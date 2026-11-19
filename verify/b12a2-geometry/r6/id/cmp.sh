#!/usr/bin/env bash
# cmp.sh <file>: print DIFF <file> when pre and now SVG differ (default flags and --theme dark)
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a2-geometry/r6
P=$V/pre/target/release/flowmaid; N=/home/user/flowmaid/target/release/flowmaid
f=$1
a=$(timeout 60 $P "$f" 2>&1 | md5sum); b=$(timeout 60 $N "$f" 2>&1 | md5sum)
[ "$a" = "$b" ] || echo "DIFF $f"
