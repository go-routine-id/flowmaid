#!/bin/bash
PRE=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b5-theme/pre/target/release/flowmaid
NOW=/home/user/flowmaid/target/release/flowmaid
f=$1
a=$(timeout 20 $PRE "$f" 2>&1 | md5sum)
b=$(timeout 20 $NOW "$f" 2>&1 | md5sum)
[ "$a" != "$b" ] && echo "DIFF $f"
exit 0
