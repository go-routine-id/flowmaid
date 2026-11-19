#!/bin/bash
f=$1; V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/small2-interactive
a=$(timeout 20 $V/head-bin "$f" 2>&1 | md5sum); b=$(timeout 20 $V/base-target/release/flowmaid "$f" 2>&1 | md5sum)
[ "$a" = "$b" ] && echo SAME || echo "DIFF $f"
