#!/bin/bash
f="$1"; W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/wt
a=$(timeout 60 $W/flowmaid-old "$f" -o /dev/stdout 2>&1 | md5sum | cut -c1-32)
b=$(timeout 60 $W/flowmaid-new "$f" -o /dev/stdout 2>&1 | md5sum | cut -c1-32)
[ "$a" = "$b" ] && echo "SAME" || echo "DIFF $f"
