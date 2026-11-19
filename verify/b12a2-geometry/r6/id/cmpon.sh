#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a2-geometry/r6
P=$V/t78-target/release/flowmaid; N=$V/on-target/release/flowmaid
f=$1
a=$(timeout 120 $P "$f" 2>&1 | md5sum); b=$(timeout 120 $N "$f" 2>&1 | md5sum)
[ "$a" = "$b" ] || echo "DIFF $f"
